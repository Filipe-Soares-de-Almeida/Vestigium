use windows::Win32::Foundation::{ERROR_INSUFFICIENT_BUFFER, NO_ERROR};
use windows::Win32::NetworkManagement::IpHelper::{
    GetExtendedTcpTable, GetExtendedUdpTable, MIB_TCP6TABLE_OWNER_PID, MIB_TCPTABLE_OWNER_PID,
    MIB_UDP6TABLE_OWNER_PID, MIB_UDPTABLE_OWNER_PID, TCP_TABLE_OWNER_PID_ALL, UDP_TABLE_OWNER_PID,
};
use windows::Win32::Networking::WinSock::{AF_INET, AF_INET6};

/// TCP and UDP endpoints, IPv4 and IPv6, owned by `pid`.
pub fn endpoints_owned_by(pid: u32) -> Result<usize, String> {
    let mut total = 0;
    for family in [AF_INET.0, AF_INET6.0] {
        // SAFETY: each closure fills the buffer it is given, up to the size it is told.
        let tcp = table(|buf, size| unsafe {
            GetExtendedTcpTable(
                buf,
                size,
                false,
                u32::from(family),
                TCP_TABLE_OWNER_PID_ALL,
                0,
            )
        })?;
        // SAFETY: as above, for the UDP table.
        let udp = table(|buf, size| unsafe {
            GetExtendedUdpTable(buf, size, false, u32::from(family), UDP_TABLE_OWNER_PID, 0)
        })?;
        // SAFETY: each buffer holds the table type its call and family produce.
        total += unsafe {
            if family == AF_INET.0 {
                owners::<MIB_TCPTABLE_OWNER_PID>(
                    &tcp,
                    |t| rows(t).iter().map(|r| r.dwOwningPid).collect(),
                    pid,
                ) + owners::<MIB_UDPTABLE_OWNER_PID>(
                    &udp,
                    |t| rows(t).iter().map(|r| r.dwOwningPid).collect(),
                    pid,
                )
            } else {
                owners::<MIB_TCP6TABLE_OWNER_PID>(
                    &tcp,
                    |t| rows(t).iter().map(|r| r.dwOwningPid).collect(),
                    pid,
                ) + owners::<MIB_UDP6TABLE_OWNER_PID>(
                    &udp,
                    |t| rows(t).iter().map(|r| r.dwOwningPid).collect(),
                    pid,
                )
            }
        };
    }
    Ok(total)
}

/// Calls an IP Helper table function twice: once for the size, once to fill.
fn table(
    call: impl Fn(Option<*mut core::ffi::c_void>, *mut u32) -> u32,
) -> Result<Vec<u64>, String> {
    let mut size = 0u32;
    let mut buf: Vec<u64> = Vec::new();
    for _ in 0..4 {
        let target = (!buf.is_empty()).then(|| buf.as_mut_ptr().cast());
        match call(target, &mut size) {
            code if code == NO_ERROR.0 && target.is_some() => return Ok(buf),
            code if code == ERROR_INSUFFICIENT_BUFFER.0 || code == NO_ERROR.0 => {
                buf = vec![0; (size as usize).div_ceil(8) + 1];
            }
            code => return Err(format!("IP Helper error {code}")),
        }
    }
    Err("IP Helper table kept growing".to_string())
}

/// Common shape of the four tables: a count followed by that many rows.
trait Table {
    type Row;
    fn count(&self) -> u32;
    fn first(&self) -> *const Self::Row;
}

macro_rules! impl_table {
    ($($table:ty => $row:ty),*) => {$(
        impl Table for $table {
            type Row = $row;
            fn count(&self) -> u32 { self.dwNumEntries }
            fn first(&self) -> *const $row { self.table.as_ptr() }
        }
    )*};
}

impl_table!(
    MIB_TCPTABLE_OWNER_PID => windows::Win32::NetworkManagement::IpHelper::MIB_TCPROW_OWNER_PID,
    MIB_TCP6TABLE_OWNER_PID => windows::Win32::NetworkManagement::IpHelper::MIB_TCP6ROW_OWNER_PID,
    MIB_UDPTABLE_OWNER_PID => windows::Win32::NetworkManagement::IpHelper::MIB_UDPROW_OWNER_PID,
    MIB_UDP6TABLE_OWNER_PID => windows::Win32::NetworkManagement::IpHelper::MIB_UDP6ROW_OWNER_PID
);

/// # Safety
/// `table` must hold `count()` rows after its header, as the IP Helper call wrote them.
unsafe fn rows<T: Table>(table: &T) -> &[T::Row] {
    // SAFETY: guaranteed by the caller.
    unsafe { std::slice::from_raw_parts(table.first(), table.count() as usize) }
}

/// # Safety
/// `buf` must hold a table of type `T` filled by the matching IP Helper call.
unsafe fn owners<T>(buf: &[u64], pids: impl Fn(&T) -> Vec<u32>, pid: u32) -> usize {
    // SAFETY: guaranteed by the caller; the buffer is 8-byte aligned.
    let table = unsafe { &*buf.as_ptr().cast::<T>() };
    pids(table)
        .into_iter()
        .filter(|&owner| owner == pid)
        .count()
}
