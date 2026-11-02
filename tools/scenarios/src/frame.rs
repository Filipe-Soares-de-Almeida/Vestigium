use std::thread;
use std::time::{Duration, Instant};

use vestigium_collect::wall_ms;
use windows::Win32::Foundation::{HMODULE, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::Graphics::Direct3D::D3D_DRIVER_TYPE_HARDWARE;
use windows::Win32::Graphics::Direct3D11::{
    D3D11_CREATE_DEVICE_FLAG, D3D11_SDK_VERSION, D3D11CreateDeviceAndSwapChain, ID3D11Device,
    ID3D11DeviceContext, ID3D11RenderTargetView, ID3D11Texture2D,
};
use windows::Win32::Graphics::Dxgi::Common::{
    DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_MODE_DESC, DXGI_SAMPLE_DESC,
};
use windows::Win32::Graphics::Dxgi::{
    DXGI_PRESENT, DXGI_SWAP_CHAIN_DESC, DXGI_SWAP_EFFECT_FLIP_DISCARD,
    DXGI_USAGE_RENDER_TARGET_OUTPUT, IDXGISwapChain,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Performance::QueryPerformanceCounter;
use windows::Win32::UI::WindowsAndMessaging::{
    CW_USEDEFAULT, CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, MSG,
    PM_REMOVE, PeekMessageW, RegisterClassW, TranslateMessage, WINDOW_EX_STYLE, WNDCLASSW,
    WS_OVERLAPPEDWINDOW, WS_VISIBLE,
};
use windows::core::w;

use crate::stopped;

const WIDTH: u32 = 640;
const HEIGHT: u32 = 360;

/// Renders a window at the display's refresh rate and stalls the render thread every `every_secs`.
pub fn hitch(seconds: u64, every_secs: u64, hitch_ms: u64) -> Result<(), String> {
    let window = create_window()?;
    let (swap, context, target) = device(window)?;
    println!(
        "Presenting at the refresh rate for {seconds} s; every {every_secs} s the render thread stalls {hitch_ms} ms."
    );
    let start = Instant::now();
    let mut next_hitch = start + Duration::from_secs(every_secs);
    let mut frames = 0u64;
    let mut hitches = 0u32;
    while start.elapsed() < Duration::from_secs(seconds) && !stopped() {
        pump();
        if Instant::now() >= next_hitch {
            let mut qpc = 0i64;
            // SAFETY: writes the counter into a local.
            let _ = unsafe { QueryPerformanceCounter(&mut qpc) };
            println!("hitch at wall_ms {} qpc {qpc}", wall_ms());
            thread::sleep(Duration::from_millis(hitch_ms));
            hitches += 1;
            next_hitch += Duration::from_secs(every_secs);
        }
        let shade = (frames % 120) as f32 / 120.0;
        // SAFETY: the context and view belong to the device created above, on this thread.
        unsafe {
            context.OMSetRenderTargets(Some(&[Some(target.clone())]), None);
            context.ClearRenderTargetView(&target, &[shade * 0.6, 0.1, 0.15, 1.0]);
        }
        // SAFETY: presents the back buffer of this thread's swap chain, waiting for one vertical blank.
        let presented = unsafe { swap.Present(1, DXGI_PRESENT(0)) };
        if presented.is_err() {
            return Err(format!("Present failed: {presented:?}"));
        }
        frames += 1;
    }
    println!("Done: {frames} frames, {hitches} hitches.");
    // SAFETY: destroys the window this thread created.
    let _ = unsafe { DestroyWindow(window) };
    Ok(())
}

fn device(
    window: HWND,
) -> Result<(IDXGISwapChain, ID3D11DeviceContext, ID3D11RenderTargetView), String> {
    let desc = DXGI_SWAP_CHAIN_DESC {
        BufferDesc: DXGI_MODE_DESC {
            Width: WIDTH,
            Height: HEIGHT,
            Format: DXGI_FORMAT_B8G8R8A8_UNORM,
            ..Default::default()
        },
        SampleDesc: DXGI_SAMPLE_DESC {
            Count: 1,
            Quality: 0,
        },
        BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
        BufferCount: 2,
        OutputWindow: window,
        Windowed: true.into(),
        SwapEffect: DXGI_SWAP_EFFECT_FLIP_DISCARD,
        Flags: 0,
    };
    let mut swap: Option<IDXGISwapChain> = None;
    let mut dev: Option<ID3D11Device> = None;
    let mut context: Option<ID3D11DeviceContext> = None;
    // SAFETY: every out pointer is a local Option that lives through the call.
    unsafe {
        D3D11CreateDeviceAndSwapChain(
            None,
            D3D_DRIVER_TYPE_HARDWARE,
            HMODULE::default(),
            D3D11_CREATE_DEVICE_FLAG(0),
            None,
            D3D11_SDK_VERSION,
            Some(&desc),
            Some(&mut swap),
            Some(&mut dev),
            None,
            Some(&mut context),
        )
    }
    .map_err(|e| e.to_string())?;
    let (swap, dev, context) = match (swap, dev, context) {
        (Some(s), Some(d), Some(c)) => (s, d, c),
        _ => return Err("the device was not created".into()),
    };
    // SAFETY: buffer 0 of a flip swap chain is its back buffer.
    let back: ID3D11Texture2D = unsafe { swap.GetBuffer(0) }.map_err(|e| e.to_string())?;
    let mut target: Option<ID3D11RenderTargetView> = None;
    // SAFETY: a default view of the back buffer, written into a local.
    unsafe { dev.CreateRenderTargetView(&back, None, Some(&mut target)) }
        .map_err(|e| e.to_string())?;
    let target = target.ok_or("no render target view")?;
    Ok((swap, context, target))
}

fn create_window() -> Result<HWND, String> {
    // SAFETY: the module handle of this executable.
    let instance = unsafe { GetModuleHandleW(None) }.map_err(|e| e.to_string())?;
    let class = WNDCLASSW {
        lpfnWndProc: Some(window_proc),
        hInstance: instance.into(),
        lpszClassName: w!("VestigiumScenarioFrames"),
        ..Default::default()
    };
    // SAFETY: registers a class whose strings are static.
    if unsafe { RegisterClassW(&class) } == 0 {
        return Err(windows::core::Error::from_thread().to_string());
    }
    // SAFETY: a plain top-level window of the class registered above, owned by this thread.
    unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE(0),
            w!("VestigiumScenarioFrames"),
            w!("Vestigium scenario: frame-hitch"),
            WS_OVERLAPPEDWINDOW | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            WIDTH as i32,
            HEIGHT as i32,
            None,
            None,
            Some(instance.into()),
            None,
        )
    }
    .map_err(|e| e.to_string())
}

fn pump() {
    let mut msg = MSG::default();
    // SAFETY: reads this thread's queue into `msg` without waiting.
    while unsafe { PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE) }.as_bool() {
        // SAFETY: `msg` was just filled by PeekMessageW.
        unsafe {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    // SAFETY: forwards every message unchanged.
    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}
