// Written by the `wire` test of the window crate; do not edit by hand.
// Refresh with `UPDATE_FIXTURES=1 cargo test -p vestigium-ui wire`.
import type { Advice, AgentLink, AgentStatus, AppFrames, Applier, BenchMode, BenchResult, Benchmark, BootDelay, BootReport, BootTime, BugCheckDto, Bus, CaptureDto, CaptureFinished, CaptureNote, CaptureRun, CaptureSource, CaptureSummary, Cause, Channel, ColorTheme, Columns, Concern, Confidence, CoreLoad, CrashControlDto, CrashKind, Culprit, DelayKind, DeleteFinished, DetailLevel, DiagnosticDetail, DiagnosticResult, DiskDetailDto, DiskDto, DiskHealth, DiskInfo, DiskLoad, DriverLatency, DumpCulprit, DumpDto, DumpKind, DumpScanFinished, DumpsDto, Episode, EpisodeDetailDto, EpisodeKind, EventMeaning, Evidence, Fact, FactKind, FaultDto, FindingDto, FindingKind, FrameHitch, Gap, GapReason, HangOutcome, HangSpan, HealthDto, HealthFact, Helped, HistoryState, Hive, HungWindow, Lanes, LanguagePref, LiveMessage, LivePoint, LongRoutine, MachineInfo, MarksDto, Measure, Media, MemoryDiagnostic, MemoryDto, MemoryError, MemoryQuick, Metric, MonitorSource, MonitorState, MonitorStatus, NextStep, NowFrame, NvmeDetail, NvmeHealth, Occurrence, OccurrenceDto, OccurrenceKind, OccurrenceSummary, PageFile, Pattern, PcState, ProbeHiccup, ProcessFacts, ProcessSample, ProcessTop, ProgramMarks, ProgramPoint, Reading, Reference, Region, Resolution, RestorePoint, Retention, RoutineKind, RoutineStats, RunAs, RunningTest, Sample, ScanFinished, ScanUnderWay, SecurityProduct, SettingChange, SettingResult, SettingValue, Settings, SettingsDto, Severity, Signature, SlowBlock, SmartAttribute, SmartPoint, SourceFailure, StartupList, Subject, SurfaceProgress, SurfaceScan, Suspect, Takes, TestData, TestFinished, TestKind, TestOutcome, TestProgress, TestRun, TestRunDto, TestSpan, TestsDto, ThemePref, TimelineDto, TrayState, Trigger, TweakChange, TweakId, TweakRow, TweakSetting, TweakSignal, TweakStatus, TweakValue, TweaksApplied, TweaksDto, UiError, UiErrorKind, VolumeCheck, VolumeDto, VolumeResult, WinEvent, Window } from "../../lib/types";
import type { CheckResult, NotesGroup, Progress, Release, UpdatePrefs } from "../../lib/update.svelte";
import type { About } from "../../screens/settings/data";

export const pcStateSamples = [
  "calm",
  "busy",
  "pressure",
  "suspended",
  "no_reading",
] satisfies PcState[];

export const historyStateSamples = [
  "ready",
  "empty",
] satisfies HistoryState[];

export const resolutionSamples = [
  "seconds",
  "minutes",
] satisfies Resolution[];

export const columnsSamples = [
  {"t":[1790000000.0,1790000001.0],"cpu":[12.5,null],"mem_used_pct":[61.5,null],"disk_active_max":[7.5,null],"disk_latency_max":[1.5,null],"gpu":[30.0,null],"probe_lag":[0.5,null],"state":["calm","suspended"]},
] satisfies Columns[];

export const timelineDtoSamples = [
  {"history":"ready","resolution":"seconds","step_ms":1000,"t":[1790000000.0,1790000001.0],"cpu":[12.5,null],"mem_used_pct":[61.5,null],"disk_active_max":[7.5,null],"disk_latency_max":[1.5,null],"gpu":[30.0,null],"probe_lag":[0.5,null],"state":["calm","suspended"]},
  {"history":"ready","resolution":"minutes","step_ms":1000,"t":[1790000000.0,1790000001.0],"cpu":[12.5,null],"mem_used_pct":[61.5,null],"disk_active_max":[7.5,null],"disk_latency_max":[1.5,null],"gpu":[30.0,null],"probe_lag":[0.5,null],"state":["calm","suspended"]},
  {"history":"empty","resolution":"seconds","step_ms":1000,"t":[1790000000.0,1790000001.0],"cpu":[12.5,null],"mem_used_pct":[61.5,null],"disk_active_max":[7.5,null],"disk_latency_max":[1.5,null],"gpu":[30.0,null],"probe_lag":[0.5,null],"state":["calm","suspended"]},
] satisfies TimelineDto[];

export const readingSamples = [
  {"ts_ms":1790000000000,"cpu_pct":12.5,"mem_used_pct":61.5,"disk_active_pct":7.5,"disk_latency_ms":1.5,"gpu_pct":30.0,"probe_lag_ms":0.5},
  {"ts_ms":1790000000000,"cpu_pct":12.5,"mem_used_pct":null,"disk_active_pct":7.5,"disk_latency_ms":1.5,"gpu_pct":null,"probe_lag_ms":0.5},
] satisfies Reading[];

export const coreLoadSamples = [
  {"cpu_pct":10.0,"dpc_pct":0.5,"isr_pct":0.25},
] satisfies CoreLoad[];

export const diskLoadSamples = [
  {"name":"0 C:","active_pct":7.5,"latency_ms":1.5,"queue_len":0.5,"bytes_per_s":1024.0},
  {"name":"0 C:","active_pct":7.5,"latency_ms":null,"queue_len":null,"bytes_per_s":null},
] satisfies DiskLoad[];

export const sampleSamples = [
  {"ts_ms":1790000000000,"cpu_pct":12.5,"dpc_pct":0.5,"isr_pct":0.25,"perf_limit_pct":100.0,"proc_perf_pct":90.0,"mem_total_bytes":17179869184,"mem_avail_bytes":8589934592,"commit_bytes":9000000000,"commit_limit_bytes":20000000000,"hard_faults_per_s":3.5,"gpu_pct":30.0,"probe_max_lag_ms":4.5,"sampler_lag_ms":0.5,"cores":[{"cpu_pct":10.0,"dpc_pct":0.5,"isr_pct":0.25}],"disks":[{"name":"0 C:","active_pct":7.5,"latency_ms":1.5,"queue_len":0.5,"bytes_per_s":1024.0}]},
  {"ts_ms":1790000000000,"cpu_pct":12.5,"dpc_pct":0.5,"isr_pct":0.25,"perf_limit_pct":null,"proc_perf_pct":null,"mem_total_bytes":17179869184,"mem_avail_bytes":8589934592,"commit_bytes":9000000000,"commit_limit_bytes":20000000000,"hard_faults_per_s":null,"gpu_pct":null,"probe_max_lag_ms":4.5,"sampler_lag_ms":0.5,"cores":[{"cpu_pct":10.0,"dpc_pct":0.5,"isr_pct":0.25}],"disks":[{"name":"0 C:","active_pct":7.5,"latency_ms":null,"queue_len":null,"bytes_per_s":null}]},
] satisfies Sample[];

export const processSampleSamples = [
  {"pid":4120,"name":"example.exe","cpu_pct":14.5,"private_bytes":123456789,"read_bps":2048.0,"write_bps":1024.0,"gpu_pct":3.5},
] satisfies ProcessSample[];

export const processTopSamples = [
  {"ts_ms":1790000000000,"entries":[{"pid":4120,"name":"example.exe","cpu_pct":14.5,"private_bytes":123456789,"read_bps":2048.0,"write_bps":1024.0,"gpu_pct":3.5}]},
] satisfies ProcessTop[];

export const hungWindowSamples = [
  {"pid":4120,"name":"example.exe"},
] satisfies HungWindow[];

export const severitySamples = [
  "Light",
  "Medium",
  "Severe",
] satisfies Severity[];

export const occurrenceKindSamples = [
  "Freeze",
  "Stutter",
  "AppHang",
  "Reported",
  "PostMortem",
] satisfies OccurrenceKind[];

export const confidenceSamples = [
  "Low",
  "Medium",
  "High",
] satisfies Confidence[];

export const channelSamples = [
  "System",
  "Application",
] satisfies Channel[];

export const hangOutcomeSamples = [
  "Recovered",
  "Closed",
  "StillHung",
] satisfies HangOutcome[];

export const gapReasonSamples = [
  "Paused",
  "Suspended",
  "Stopped",
  "Crashed",
  "ClockChanged",
  "NotSaved",
] satisfies GapReason[];

export const mediaSamples = [
  "Ssd",
  "Hdd",
  "Unknown",
] satisfies Media[];

export const metricSamples = [
  "DiskLatencyMs",
  "DiskQueue",
  "DiskActivePct",
  "MemAvailPct",
  "HardFaultsPerS",
  "CpuPct",
  "CoreDpcIsrPct",
  "GpuPct",
  "PerfLimitPct",
  "PrivateBytes",
  "IoBytesPerS",
] satisfies Metric[];

export const eventMeaningSamples = [
  "StorageReset",
  "DiskRetry",
  "DiskBadBlock",
  "PagingError",
  "DisplayReset",
  "HardwareError",
  "PowerLoss",
  "UnexpectedShutdown",
  "BugCheck",
  "CpuFirmwareLimit",
  "LowVirtualMemory",
  "AppHang",
  "AppCrash",
  "ServiceCrash",
  "NoCrashDump",
  "MemoryDiagnostic",
] satisfies EventMeaning[];

export const nextStepSamples = [
  "UpdateStorageDriver",
  "CheckDiskCable",
  "RunDiskTest",
  "FindDiskHeavyApp",
  "CloseMemoryHeavyApp",
  "CheckCooling",
  "CheckPowerPlan",
  "UpdateGpuDriver",
  "EnableSensitiveMode",
  "DeepCapture",
  "ReportToAppVendor",
  "UpdateDriver",
  "BackUpNow",
  "ReplaceDisk",
  "FreeDiskSpace",
  "LetWindowsManagePageFile",
  "CheckHardwareStability",
  "KeepOneAntivirus",
  "ReviewStartupApps",
  "WaitForShutdown",
  "MoveWindowsToSsd",
  "RunMemoryDiagnostic",
  "RunMemTest86",
  "RepairVolume",
  "ReadCrashDump",
] satisfies NextStep[];

export const causeSamples = [
  {"Storage":{"disk":"0 C:"}},
  {"DiskIo":{"disk":"0 C:","hdd":true,"process":"example.exe"}},
  {"Memory":{"process":"example.exe"}},
  {"Cpu":{"processes":["example.exe"]}},
  {"Driver":{"core":3}},
  "GpuDriver",
  "Throttling",
  {"App":{"name":"example.exe"}},
  "Unclear",
  {"DriverModule":{"module":"example.sys","intermediary":false}},
  {"Frames":{"app":"example.exe"}},
  {"BlueScreen":{"code":159}},
  "Unresponsive",
  "SleepTransition",
  {"PowerCut":{"no_dump":true}},
  "HardwareFault",
  "NotMeasured",
  {"Storage":{"disk":null}},
  {"DiskIo":{"disk":"1 D:","hdd":false,"process":null}},
  {"Memory":{"process":null}},
  {"DriverModule":{"module":"example.sys","intermediary":true}},
  {"PowerCut":{"no_dump":false}},
] satisfies Cause[];

export const crashKindSamples = [
  {"BlueScreen":{"code":80,"params":[1,2,3,4]}},
  "PowerButton",
  "WhileShuttingDown",
  "WhileSleeping",
  "NoTrace",
] satisfies CrashKind[];

export const winEventSamples = [
  {"ts_ms":1790000000000,"channel":"System","provider":"example-provider","id":129,"record_id":99001,"level":2,"meaning":"StorageReset","data_xml":"<Data Name=\"Example\">1</Data>"},
  {"ts_ms":1790000000000,"channel":"System","provider":"example-provider","id":129,"record_id":99001,"level":2,"meaning":"DiskRetry","data_xml":"<Data Name=\"Example\">1</Data>"},
  {"ts_ms":1790000000000,"channel":"System","provider":"example-provider","id":129,"record_id":99001,"level":2,"meaning":"DiskBadBlock","data_xml":"<Data Name=\"Example\">1</Data>"},
  {"ts_ms":1790000000000,"channel":"System","provider":"example-provider","id":129,"record_id":99001,"level":2,"meaning":"PagingError","data_xml":"<Data Name=\"Example\">1</Data>"},
  {"ts_ms":1790000000000,"channel":"System","provider":"example-provider","id":129,"record_id":99001,"level":2,"meaning":"DisplayReset","data_xml":"<Data Name=\"Example\">1</Data>"},
  {"ts_ms":1790000000000,"channel":"System","provider":"example-provider","id":129,"record_id":99001,"level":2,"meaning":"HardwareError","data_xml":"<Data Name=\"Example\">1</Data>"},
  {"ts_ms":1790000000000,"channel":"System","provider":"example-provider","id":129,"record_id":99001,"level":2,"meaning":"PowerLoss","data_xml":"<Data Name=\"Example\">1</Data>"},
  {"ts_ms":1790000000000,"channel":"System","provider":"example-provider","id":129,"record_id":99001,"level":2,"meaning":"UnexpectedShutdown","data_xml":"<Data Name=\"Example\">1</Data>"},
  {"ts_ms":1790000000000,"channel":"System","provider":"example-provider","id":129,"record_id":99001,"level":2,"meaning":"BugCheck","data_xml":"<Data Name=\"Example\">1</Data>"},
  {"ts_ms":1790000000000,"channel":"System","provider":"example-provider","id":129,"record_id":99001,"level":2,"meaning":"CpuFirmwareLimit","data_xml":"<Data Name=\"Example\">1</Data>"},
  {"ts_ms":1790000000000,"channel":"System","provider":"example-provider","id":129,"record_id":99001,"level":2,"meaning":"LowVirtualMemory","data_xml":"<Data Name=\"Example\">1</Data>"},
  {"ts_ms":1790000000000,"channel":"System","provider":"example-provider","id":129,"record_id":99001,"level":2,"meaning":"AppHang","data_xml":"<Data Name=\"Example\">1</Data>"},
  {"ts_ms":1790000000000,"channel":"System","provider":"example-provider","id":129,"record_id":99001,"level":2,"meaning":"AppCrash","data_xml":"<Data Name=\"Example\">1</Data>"},
  {"ts_ms":1790000000000,"channel":"System","provider":"example-provider","id":129,"record_id":99001,"level":2,"meaning":"ServiceCrash","data_xml":"<Data Name=\"Example\">1</Data>"},
  {"ts_ms":1790000000000,"channel":"System","provider":"example-provider","id":129,"record_id":99001,"level":2,"meaning":"NoCrashDump","data_xml":"<Data Name=\"Example\">1</Data>"},
  {"ts_ms":1790000000000,"channel":"System","provider":"example-provider","id":129,"record_id":99001,"level":2,"meaning":"MemoryDiagnostic","data_xml":"<Data Name=\"Example\">1</Data>"},
  {"ts_ms":1790000000000,"channel":"Application","provider":"example-provider","id":129,"record_id":99001,"level":2,"meaning":"StorageReset","data_xml":"<Data Name=\"Example\">1</Data>"},
  {"ts_ms":1790000000000,"channel":"Application","provider":"example-provider","id":129,"record_id":99001,"level":2,"meaning":"DiskRetry","data_xml":"<Data Name=\"Example\">1</Data>"},
  {"ts_ms":1790000000000,"channel":"Application","provider":"example-provider","id":129,"record_id":99001,"level":2,"meaning":"DiskBadBlock","data_xml":"<Data Name=\"Example\">1</Data>"},
  {"ts_ms":1790000000000,"channel":"Application","provider":"example-provider","id":129,"record_id":99001,"level":2,"meaning":"PagingError","data_xml":"<Data Name=\"Example\">1</Data>"},
  {"ts_ms":1790000000000,"channel":"Application","provider":"example-provider","id":129,"record_id":99001,"level":2,"meaning":"DisplayReset","data_xml":"<Data Name=\"Example\">1</Data>"},
  {"ts_ms":1790000000000,"channel":"Application","provider":"example-provider","id":129,"record_id":99001,"level":2,"meaning":"HardwareError","data_xml":"<Data Name=\"Example\">1</Data>"},
  {"ts_ms":1790000000000,"channel":"Application","provider":"example-provider","id":129,"record_id":99001,"level":2,"meaning":"PowerLoss","data_xml":"<Data Name=\"Example\">1</Data>"},
  {"ts_ms":1790000000000,"channel":"Application","provider":"example-provider","id":129,"record_id":99001,"level":2,"meaning":"UnexpectedShutdown","data_xml":"<Data Name=\"Example\">1</Data>"},
  {"ts_ms":1790000000000,"channel":"Application","provider":"example-provider","id":129,"record_id":99001,"level":2,"meaning":"BugCheck","data_xml":"<Data Name=\"Example\">1</Data>"},
  {"ts_ms":1790000000000,"channel":"Application","provider":"example-provider","id":129,"record_id":99001,"level":2,"meaning":"CpuFirmwareLimit","data_xml":"<Data Name=\"Example\">1</Data>"},
  {"ts_ms":1790000000000,"channel":"Application","provider":"example-provider","id":129,"record_id":99001,"level":2,"meaning":"LowVirtualMemory","data_xml":"<Data Name=\"Example\">1</Data>"},
  {"ts_ms":1790000000000,"channel":"Application","provider":"example-provider","id":129,"record_id":99001,"level":2,"meaning":"AppHang","data_xml":"<Data Name=\"Example\">1</Data>"},
  {"ts_ms":1790000000000,"channel":"Application","provider":"example-provider","id":129,"record_id":99001,"level":2,"meaning":"AppCrash","data_xml":"<Data Name=\"Example\">1</Data>"},
  {"ts_ms":1790000000000,"channel":"Application","provider":"example-provider","id":129,"record_id":99001,"level":2,"meaning":"ServiceCrash","data_xml":"<Data Name=\"Example\">1</Data>"},
  {"ts_ms":1790000000000,"channel":"Application","provider":"example-provider","id":129,"record_id":99001,"level":2,"meaning":"NoCrashDump","data_xml":"<Data Name=\"Example\">1</Data>"},
  {"ts_ms":1790000000000,"channel":"Application","provider":"example-provider","id":129,"record_id":99001,"level":2,"meaning":"MemoryDiagnostic","data_xml":"<Data Name=\"Example\">1</Data>"},
] satisfies WinEvent[];

export const evidenceSamples = [
  {"Event":{"ts_ms":1790000000000,"provider":"example-provider","id":129,"meaning":"StorageReset"}},
  {"Metric":{"ts_ms":1790000000000,"metric":"DiskLatencyMs","value":450.5,"threshold":100.0,"subject":"0 C:"}},
  {"Process":{"pid":4120,"name":"example.exe","metric":"PrivateBytes","value":1500000000.0}},
  {"Hang":{"name":"example.exe","duration_ms":31000}},
  {"Routine":{"module":"example.sys","kind":"Dpc","count":120,"max_us":900,"at_least_100us":40,"at_least_1ms":2,"core_pct":4.5}},
  {"LongRoutine":{"ts_ms":1790000000000,"module":"example.sys","kind":"Isr","us":1500}},
  {"Frames":{"app":"example.exe","frames":600,"hitches":12,"worst_ms":80.5,"median_ms":8.5}},
  {"Hitch":{"ts_ms":1790000000000,"app":"example.exe","frame_ms":70.5}},
  {"Hiccup":{"ts_ms":1790000000000,"lag_ms":250.5}},
  {"CaptureLoss":{"lost_events":17}},
  {"Metric":{"ts_ms":1790000000000,"metric":"CpuPct","value":99.5,"threshold":90.0,"subject":null}},
  {"LongRoutine":{"ts_ms":1790000000000,"module":"example.sys","kind":"Dpc","us":800}},
  {"LongRoutine":{"ts_ms":1790000000000,"module":"example.sys","kind":"Isr","us":800}},
] satisfies Evidence[];

export const suspectSamples = [
  {"cause":{"Storage":{"disk":"0 C:"}},"confidence":"Low","evidence":[],"next_steps":[]},
  {"cause":{"DiskIo":{"disk":"0 C:","hdd":true,"process":"example.exe"}},"confidence":"Medium","evidence":[],"next_steps":[]},
  {"cause":{"Memory":{"process":"example.exe"}},"confidence":"High","evidence":[],"next_steps":[]},
  {"cause":{"Cpu":{"processes":["example.exe"]}},"confidence":"Low","evidence":[],"next_steps":[]},
  {"cause":{"Driver":{"core":3}},"confidence":"Medium","evidence":[],"next_steps":[]},
  {"cause":"GpuDriver","confidence":"High","evidence":[],"next_steps":[]},
  {"cause":"Throttling","confidence":"Low","evidence":[],"next_steps":[]},
  {"cause":{"App":{"name":"example.exe"}},"confidence":"Medium","evidence":[],"next_steps":[]},
  {"cause":"Unclear","confidence":"High","evidence":[],"next_steps":[]},
  {"cause":{"DriverModule":{"module":"example.sys","intermediary":false}},"confidence":"Low","evidence":[],"next_steps":[]},
  {"cause":{"Frames":{"app":"example.exe"}},"confidence":"Medium","evidence":[],"next_steps":[]},
  {"cause":{"BlueScreen":{"code":159}},"confidence":"High","evidence":[],"next_steps":[]},
  {"cause":"Unresponsive","confidence":"Low","evidence":[],"next_steps":[]},
  {"cause":"SleepTransition","confidence":"Medium","evidence":[],"next_steps":[]},
  {"cause":{"PowerCut":{"no_dump":true}},"confidence":"High","evidence":[],"next_steps":[]},
  {"cause":"HardwareFault","confidence":"Low","evidence":[],"next_steps":[]},
  {"cause":"NotMeasured","confidence":"Medium","evidence":[],"next_steps":[]},
  {"cause":{"Storage":{"disk":null}},"confidence":"High","evidence":[],"next_steps":[]},
  {"cause":{"DiskIo":{"disk":"1 D:","hdd":false,"process":null}},"confidence":"Low","evidence":[],"next_steps":[]},
  {"cause":{"Memory":{"process":null}},"confidence":"Medium","evidence":[],"next_steps":[]},
  {"cause":{"DriverModule":{"module":"example.sys","intermediary":true}},"confidence":"High","evidence":[],"next_steps":[]},
  {"cause":{"PowerCut":{"no_dump":false}},"confidence":"Low","evidence":[],"next_steps":[]},
  {"cause":"Unclear","confidence":"Medium","evidence":[{"Event":{"ts_ms":1790000000000,"provider":"example-provider","id":129,"meaning":"StorageReset"}},{"Metric":{"ts_ms":1790000000000,"metric":"DiskLatencyMs","value":450.5,"threshold":100.0,"subject":"0 C:"}},{"Process":{"pid":4120,"name":"example.exe","metric":"PrivateBytes","value":1500000000.0}},{"Hang":{"name":"example.exe","duration_ms":31000}},{"Routine":{"module":"example.sys","kind":"Dpc","count":120,"max_us":900,"at_least_100us":40,"at_least_1ms":2,"core_pct":4.5}},{"LongRoutine":{"ts_ms":1790000000000,"module":"example.sys","kind":"Isr","us":1500}},{"Frames":{"app":"example.exe","frames":600,"hitches":12,"worst_ms":80.5,"median_ms":8.5}},{"Hitch":{"ts_ms":1790000000000,"app":"example.exe","frame_ms":70.5}},{"Hiccup":{"ts_ms":1790000000000,"lag_ms":250.5}},{"CaptureLoss":{"lost_events":17}},{"Metric":{"ts_ms":1790000000000,"metric":"CpuPct","value":99.5,"threshold":90.0,"subject":null}},{"LongRoutine":{"ts_ms":1790000000000,"module":"example.sys","kind":"Dpc","us":800}},{"LongRoutine":{"ts_ms":1790000000000,"module":"example.sys","kind":"Isr","us":800}}],"next_steps":["UpdateStorageDriver","CheckDiskCable","RunDiskTest","FindDiskHeavyApp","CloseMemoryHeavyApp","CheckCooling","CheckPowerPlan","UpdateGpuDriver","EnableSensitiveMode","DeepCapture","ReportToAppVendor","UpdateDriver","BackUpNow","ReplaceDisk","FreeDiskSpace","LetWindowsManagePageFile","CheckHardwareStability","KeepOneAntivirus","ReviewStartupApps","WaitForShutdown","MoveWindowsToSsd","RunMemoryDiagnostic","RunMemTest86","RepairVolume","ReadCrashDump"]},
] satisfies Suspect[];

export const triggerSamples = [
  {"Freeze":{"start_ms":1790000000000,"end_ms":1790000005000,"lag_ms":5000.5}},
  {"Stutter":{"start_ms":1790000000000,"end_ms":1790000004000,"count":6,"worst_ms":220.5}},
  {"AppHang":{"pid":4120,"name":"example.exe","start_ms":1790000000000,"end_ms":1790000030000,"outcome":"Recovered"}},
  {"Reported":{"event":{"ts_ms":1790000000000,"channel":"Application","provider":"example-provider","id":129,"record_id":99001,"level":2,"meaning":"AppCrash","data_xml":"<Data Name=\"Example\">1</Data>"}}},
  {"Crash":{"last_ms":1790000000000,"down_ms":1790000300000,"kind":"NoTrace"}},
  {"AppHang":{"pid":4120,"name":"example.exe","start_ms":1790000000000,"end_ms":1790000030000,"outcome":"Recovered"}},
  {"AppHang":{"pid":4120,"name":"example.exe","start_ms":1790000000000,"end_ms":1790000030000,"outcome":"Closed"}},
  {"AppHang":{"pid":4120,"name":"example.exe","start_ms":1790000000000,"end_ms":1790000030000,"outcome":"StillHung"}},
  {"Crash":{"last_ms":1790000000000,"down_ms":1790000300000,"kind":{"BlueScreen":{"code":80,"params":[1,2,3,4]}}}},
  {"Crash":{"last_ms":1790000000000,"down_ms":1790000300000,"kind":"PowerButton"}},
  {"Crash":{"last_ms":1790000000000,"down_ms":1790000300000,"kind":"WhileShuttingDown"}},
  {"Crash":{"last_ms":1790000000000,"down_ms":1790000300000,"kind":"WhileSleeping"}},
  {"Crash":{"last_ms":1790000000000,"down_ms":1790000300000,"kind":"NoTrace"}},
] satisfies Trigger[];

export const gapSamples = [
  {"start_ms":1790000000000,"end_ms":1790000060000,"reason":"Paused"},
  {"start_ms":1790000000000,"end_ms":1790000060000,"reason":"Suspended"},
  {"start_ms":1790000000000,"end_ms":1790000060000,"reason":"Stopped"},
  {"start_ms":1790000000000,"end_ms":1790000060000,"reason":"Crashed"},
  {"start_ms":1790000000000,"end_ms":1790000060000,"reason":"ClockChanged"},
  {"start_ms":1790000000000,"end_ms":1790000060000,"reason":"NotSaved"},
] satisfies Gap[];

export const hangSpanSamples = [
  {"pid":4120,"name":"example.exe","start_ms":1790000000000,"end_ms":1790000030000,"outcome":"Recovered"},
  {"pid":4120,"name":"example.exe","start_ms":1790000000000,"end_ms":1790000030000,"outcome":"Closed"},
  {"pid":4120,"name":"example.exe","start_ms":1790000000000,"end_ms":1790000030000,"outcome":"StillHung"},
] satisfies HangSpan[];

export const diskInfoSamples = [
  {"name":"0 C:","media":"Ssd"},
  {"name":"0 C:","media":"Hdd"},
  {"name":"0 C:","media":"Unknown"},
] satisfies DiskInfo[];

export const machineInfoSamples = [
  {"logical_cpus":8,"mem_total_bytes":17179869184,"disks":[{"name":"0 C:","media":"Ssd"},{"name":"0 C:","media":"Hdd"},{"name":"0 C:","media":"Unknown"}]},
] satisfies MachineInfo[];

export const windowSamples = [
  {"from_ms":1789999900000,"to_ms":1790000100000,"samples":[{"ts_ms":1790000000000,"cpu_pct":12.5,"dpc_pct":0.5,"isr_pct":0.25,"perf_limit_pct":100.0,"proc_perf_pct":90.0,"mem_total_bytes":17179869184,"mem_avail_bytes":8589934592,"commit_bytes":9000000000,"commit_limit_bytes":20000000000,"hard_faults_per_s":3.5,"gpu_pct":30.0,"probe_max_lag_ms":4.5,"sampler_lag_ms":0.5,"cores":[{"cpu_pct":10.0,"dpc_pct":0.5,"isr_pct":0.25}],"disks":[{"name":"0 C:","active_pct":7.5,"latency_ms":1.5,"queue_len":0.5,"bytes_per_s":1024.0}]}],"tops":[{"ts_ms":1790000000000,"entries":[{"pid":4120,"name":"example.exe","cpu_pct":14.5,"private_bytes":123456789,"read_bps":2048.0,"write_bps":1024.0,"gpu_pct":3.5}]}],"events":[{"ts_ms":1790000000000,"channel":"System","provider":"example-provider","id":129,"record_id":99001,"level":2,"meaning":"StorageReset","data_xml":"<Data Name=\"Example\">1</Data>"},{"ts_ms":1790000000000,"channel":"Application","provider":"example-provider","id":129,"record_id":99001,"level":2,"meaning":"StorageReset","data_xml":"<Data Name=\"Example\">1</Data>"}],"hangs":[{"pid":4120,"name":"example.exe","start_ms":1790000000000,"end_ms":1790000030000,"outcome":"Recovered"},{"pid":4120,"name":"example.exe","start_ms":1790000000000,"end_ms":1790000030000,"outcome":"Closed"},{"pid":4120,"name":"example.exe","start_ms":1790000000000,"end_ms":1790000030000,"outcome":"StillHung"}],"gaps":[{"start_ms":1790000000000,"end_ms":1790000060000,"reason":"Paused"},{"start_ms":1790000000000,"end_ms":1790000060000,"reason":"Suspended"},{"start_ms":1790000000000,"end_ms":1790000060000,"reason":"Stopped"},{"start_ms":1790000000000,"end_ms":1790000060000,"reason":"Crashed"},{"start_ms":1790000000000,"end_ms":1790000060000,"reason":"ClockChanged"},{"start_ms":1790000000000,"end_ms":1790000060000,"reason":"NotSaved"}],"machine":{"logical_cpus":8,"mem_total_bytes":17179869184,"disks":[{"name":"0 C:","media":"Ssd"},{"name":"0 C:","media":"Hdd"},{"name":"0 C:","media":"Unknown"}]}},
  {"from_ms":1789999900000,"to_ms":1790000100000,"samples":[{"ts_ms":1790000000000,"cpu_pct":12.5,"dpc_pct":0.5,"isr_pct":0.25,"perf_limit_pct":null,"proc_perf_pct":null,"mem_total_bytes":17179869184,"mem_avail_bytes":8589934592,"commit_bytes":9000000000,"commit_limit_bytes":20000000000,"hard_faults_per_s":null,"gpu_pct":null,"probe_max_lag_ms":4.5,"sampler_lag_ms":0.5,"cores":[{"cpu_pct":10.0,"dpc_pct":0.5,"isr_pct":0.25}],"disks":[{"name":"0 C:","active_pct":7.5,"latency_ms":null,"queue_len":null,"bytes_per_s":null}]}],"tops":[{"ts_ms":1790000000000,"entries":[{"pid":4120,"name":"example.exe","cpu_pct":14.5,"private_bytes":123456789,"read_bps":2048.0,"write_bps":1024.0,"gpu_pct":3.5}]}],"events":[{"ts_ms":1790000000000,"channel":"System","provider":"example-provider","id":129,"record_id":99001,"level":2,"meaning":"StorageReset","data_xml":"<Data Name=\"Example\">1</Data>"},{"ts_ms":1790000000000,"channel":"Application","provider":"example-provider","id":129,"record_id":99001,"level":2,"meaning":"StorageReset","data_xml":"<Data Name=\"Example\">1</Data>"}],"hangs":[{"pid":4120,"name":"example.exe","start_ms":1790000000000,"end_ms":1790000030000,"outcome":"Recovered"},{"pid":4120,"name":"example.exe","start_ms":1790000000000,"end_ms":1790000030000,"outcome":"Closed"},{"pid":4120,"name":"example.exe","start_ms":1790000000000,"end_ms":1790000030000,"outcome":"StillHung"}],"gaps":[{"start_ms":1790000000000,"end_ms":1790000060000,"reason":"Paused"},{"start_ms":1790000000000,"end_ms":1790000060000,"reason":"Suspended"},{"start_ms":1790000000000,"end_ms":1790000060000,"reason":"Stopped"},{"start_ms":1790000000000,"end_ms":1790000060000,"reason":"Crashed"},{"start_ms":1790000000000,"end_ms":1790000060000,"reason":"ClockChanged"},{"start_ms":1790000000000,"end_ms":1790000060000,"reason":"NotSaved"}],"machine":{"logical_cpus":8,"mem_total_bytes":17179869184,"disks":[{"name":"0 C:","media":"Ssd"},{"name":"0 C:","media":"Hdd"},{"name":"0 C:","media":"Unknown"}]}},
] satisfies Window[];

export const factKindSamples = [
  {"FreezeStarted":{"duration_ms":5000.5}},
  "FreezeEnded",
  {"Stutter":{"count":6,"worst_ms":220.5}},
  {"AppHung":{"name":"example.exe"}},
  {"AppRecovered":{"name":"example.exe"}},
  {"AppClosedWhileHung":{"name":"example.exe"}},
  {"Event":{"meaning":"DiskRetry","provider":"example-provider","id":153}},
  {"Crossed":{"metric":"MemAvailPct","value":4.5,"threshold":10.0,"subject":"0 C:"}},
  {"Busiest":{"metric":"CpuPct","name":"example.exe","value":88.5}},
  {"Gap":{"reason":"ClockChanged"}},
  "LastWatched",
  {"WentDown":{"kind":"PowerButton"}},
  {"EventsDropped":{"count":40}},
  {"AppStillHung":{"name":"example.exe"}},
  {"Crossed":{"metric":"CpuPct","value":99.5,"threshold":90.0,"subject":null}},
  {"Gap":{"reason":"Paused"}},
  {"Gap":{"reason":"Suspended"}},
  {"Gap":{"reason":"Stopped"}},
  {"Gap":{"reason":"Crashed"}},
  {"Gap":{"reason":"ClockChanged"}},
  {"Gap":{"reason":"NotSaved"}},
  {"WentDown":{"kind":{"BlueScreen":{"code":80,"params":[1,2,3,4]}}}},
  {"WentDown":{"kind":"PowerButton"}},
  {"WentDown":{"kind":"WhileShuttingDown"}},
  {"WentDown":{"kind":"WhileSleeping"}},
  {"WentDown":{"kind":"NoTrace"}},
  {"Busiest":{"metric":"DiskLatencyMs","name":"example.exe","value":50.5}},
  {"Busiest":{"metric":"DiskQueue","name":"example.exe","value":50.5}},
  {"Busiest":{"metric":"DiskActivePct","name":"example.exe","value":50.5}},
  {"Busiest":{"metric":"MemAvailPct","name":"example.exe","value":50.5}},
  {"Busiest":{"metric":"HardFaultsPerS","name":"example.exe","value":50.5}},
  {"Busiest":{"metric":"CpuPct","name":"example.exe","value":50.5}},
  {"Busiest":{"metric":"CoreDpcIsrPct","name":"example.exe","value":50.5}},
  {"Busiest":{"metric":"GpuPct","name":"example.exe","value":50.5}},
  {"Busiest":{"metric":"PerfLimitPct","name":"example.exe","value":50.5}},
  {"Busiest":{"metric":"PrivateBytes","name":"example.exe","value":50.5}},
  {"Busiest":{"metric":"IoBytesPerS","name":"example.exe","value":50.5}},
  {"Event":{"meaning":"StorageReset","provider":"example-provider","id":1}},
  {"Event":{"meaning":"DiskRetry","provider":"example-provider","id":1}},
  {"Event":{"meaning":"DiskBadBlock","provider":"example-provider","id":1}},
  {"Event":{"meaning":"PagingError","provider":"example-provider","id":1}},
  {"Event":{"meaning":"DisplayReset","provider":"example-provider","id":1}},
  {"Event":{"meaning":"HardwareError","provider":"example-provider","id":1}},
  {"Event":{"meaning":"PowerLoss","provider":"example-provider","id":1}},
  {"Event":{"meaning":"UnexpectedShutdown","provider":"example-provider","id":1}},
  {"Event":{"meaning":"BugCheck","provider":"example-provider","id":1}},
  {"Event":{"meaning":"CpuFirmwareLimit","provider":"example-provider","id":1}},
  {"Event":{"meaning":"LowVirtualMemory","provider":"example-provider","id":1}},
  {"Event":{"meaning":"AppHang","provider":"example-provider","id":1}},
  {"Event":{"meaning":"AppCrash","provider":"example-provider","id":1}},
  {"Event":{"meaning":"ServiceCrash","provider":"example-provider","id":1}},
  {"Event":{"meaning":"NoCrashDump","provider":"example-provider","id":1}},
  {"Event":{"meaning":"MemoryDiagnostic","provider":"example-provider","id":1}},
] satisfies FactKind[];

export const factSamples = [
  {"at_ms":1790000001000,"kind":{"FreezeStarted":{"duration_ms":5000.5}}},
  {"at_ms":1790000001000,"kind":"FreezeEnded"},
  {"at_ms":1790000001000,"kind":{"Stutter":{"count":6,"worst_ms":220.5}}},
  {"at_ms":1790000001000,"kind":{"AppHung":{"name":"example.exe"}}},
  {"at_ms":1790000001000,"kind":{"AppRecovered":{"name":"example.exe"}}},
  {"at_ms":1790000001000,"kind":{"AppClosedWhileHung":{"name":"example.exe"}}},
  {"at_ms":1790000001000,"kind":{"Event":{"meaning":"DiskRetry","provider":"example-provider","id":153}}},
  {"at_ms":1790000001000,"kind":{"Crossed":{"metric":"MemAvailPct","value":4.5,"threshold":10.0,"subject":"0 C:"}}},
  {"at_ms":1790000001000,"kind":{"Busiest":{"metric":"CpuPct","name":"example.exe","value":88.5}}},
  {"at_ms":1790000001000,"kind":{"Gap":{"reason":"ClockChanged"}}},
  {"at_ms":1790000001000,"kind":"LastWatched"},
  {"at_ms":1790000001000,"kind":{"WentDown":{"kind":"PowerButton"}}},
  {"at_ms":1790000001000,"kind":{"EventsDropped":{"count":40}}},
  {"at_ms":1790000001000,"kind":{"AppStillHung":{"name":"example.exe"}}},
  {"at_ms":1790000001000,"kind":{"Crossed":{"metric":"CpuPct","value":99.5,"threshold":90.0,"subject":null}}},
  {"at_ms":1790000001000,"kind":{"Gap":{"reason":"Paused"}}},
  {"at_ms":1790000001000,"kind":{"Gap":{"reason":"Suspended"}}},
  {"at_ms":1790000001000,"kind":{"Gap":{"reason":"Stopped"}}},
  {"at_ms":1790000001000,"kind":{"Gap":{"reason":"Crashed"}}},
  {"at_ms":1790000001000,"kind":{"Gap":{"reason":"ClockChanged"}}},
  {"at_ms":1790000001000,"kind":{"Gap":{"reason":"NotSaved"}}},
  {"at_ms":1790000001000,"kind":{"WentDown":{"kind":{"BlueScreen":{"code":80,"params":[1,2,3,4]}}}}},
  {"at_ms":1790000001000,"kind":{"WentDown":{"kind":"PowerButton"}}},
  {"at_ms":1790000001000,"kind":{"WentDown":{"kind":"WhileShuttingDown"}}},
  {"at_ms":1790000001000,"kind":{"WentDown":{"kind":"WhileSleeping"}}},
  {"at_ms":1790000001000,"kind":{"WentDown":{"kind":"NoTrace"}}},
  {"at_ms":1790000001000,"kind":{"Busiest":{"metric":"DiskLatencyMs","name":"example.exe","value":50.5}}},
  {"at_ms":1790000001000,"kind":{"Busiest":{"metric":"DiskQueue","name":"example.exe","value":50.5}}},
  {"at_ms":1790000001000,"kind":{"Busiest":{"metric":"DiskActivePct","name":"example.exe","value":50.5}}},
  {"at_ms":1790000001000,"kind":{"Busiest":{"metric":"MemAvailPct","name":"example.exe","value":50.5}}},
  {"at_ms":1790000001000,"kind":{"Busiest":{"metric":"HardFaultsPerS","name":"example.exe","value":50.5}}},
  {"at_ms":1790000001000,"kind":{"Busiest":{"metric":"CpuPct","name":"example.exe","value":50.5}}},
  {"at_ms":1790000001000,"kind":{"Busiest":{"metric":"CoreDpcIsrPct","name":"example.exe","value":50.5}}},
  {"at_ms":1790000001000,"kind":{"Busiest":{"metric":"GpuPct","name":"example.exe","value":50.5}}},
  {"at_ms":1790000001000,"kind":{"Busiest":{"metric":"PerfLimitPct","name":"example.exe","value":50.5}}},
  {"at_ms":1790000001000,"kind":{"Busiest":{"metric":"PrivateBytes","name":"example.exe","value":50.5}}},
  {"at_ms":1790000001000,"kind":{"Busiest":{"metric":"IoBytesPerS","name":"example.exe","value":50.5}}},
  {"at_ms":1790000001000,"kind":{"Event":{"meaning":"StorageReset","provider":"example-provider","id":1}}},
  {"at_ms":1790000001000,"kind":{"Event":{"meaning":"DiskRetry","provider":"example-provider","id":1}}},
  {"at_ms":1790000001000,"kind":{"Event":{"meaning":"DiskBadBlock","provider":"example-provider","id":1}}},
  {"at_ms":1790000001000,"kind":{"Event":{"meaning":"PagingError","provider":"example-provider","id":1}}},
  {"at_ms":1790000001000,"kind":{"Event":{"meaning":"DisplayReset","provider":"example-provider","id":1}}},
  {"at_ms":1790000001000,"kind":{"Event":{"meaning":"HardwareError","provider":"example-provider","id":1}}},
  {"at_ms":1790000001000,"kind":{"Event":{"meaning":"PowerLoss","provider":"example-provider","id":1}}},
  {"at_ms":1790000001000,"kind":{"Event":{"meaning":"UnexpectedShutdown","provider":"example-provider","id":1}}},
  {"at_ms":1790000001000,"kind":{"Event":{"meaning":"BugCheck","provider":"example-provider","id":1}}},
  {"at_ms":1790000001000,"kind":{"Event":{"meaning":"CpuFirmwareLimit","provider":"example-provider","id":1}}},
  {"at_ms":1790000001000,"kind":{"Event":{"meaning":"LowVirtualMemory","provider":"example-provider","id":1}}},
  {"at_ms":1790000001000,"kind":{"Event":{"meaning":"AppHang","provider":"example-provider","id":1}}},
  {"at_ms":1790000001000,"kind":{"Event":{"meaning":"AppCrash","provider":"example-provider","id":1}}},
  {"at_ms":1790000001000,"kind":{"Event":{"meaning":"ServiceCrash","provider":"example-provider","id":1}}},
  {"at_ms":1790000001000,"kind":{"Event":{"meaning":"NoCrashDump","provider":"example-provider","id":1}}},
  {"at_ms":1790000001000,"kind":{"Event":{"meaning":"MemoryDiagnostic","provider":"example-provider","id":1}}},
] satisfies Fact[];

export const occurrenceSummarySamples = [
  {"id":1,"kind":"Freeze","start_ms":1790000000000,"end_ms":1790000005000,"severity":"Light","cause":"Unclear"},
  {"id":2,"kind":"Stutter","start_ms":1790000000000,"end_ms":1790000005000,"severity":"Medium","cause":"Unclear"},
  {"id":3,"kind":"AppHang","start_ms":1790000000000,"end_ms":1790000005000,"severity":"Severe","cause":"Unclear"},
  {"id":4,"kind":"Reported","start_ms":1790000000000,"end_ms":1790000005000,"severity":"Light","cause":"Unclear"},
  {"id":5,"kind":"PostMortem","start_ms":1790000000000,"end_ms":1790000005000,"severity":"Medium","cause":"Unclear"},
  {"id":9,"kind":"Freeze","start_ms":1790000000000,"end_ms":1790000005000,"severity":"Medium","cause":{"Storage":{"disk":"0 C:"}}},
  {"id":9,"kind":"Freeze","start_ms":1790000000000,"end_ms":1790000005000,"severity":"Medium","cause":{"DiskIo":{"disk":"0 C:","hdd":true,"process":"example.exe"}}},
  {"id":9,"kind":"Freeze","start_ms":1790000000000,"end_ms":1790000005000,"severity":"Medium","cause":{"Memory":{"process":"example.exe"}}},
  {"id":9,"kind":"Freeze","start_ms":1790000000000,"end_ms":1790000005000,"severity":"Medium","cause":{"Cpu":{"processes":["example.exe"]}}},
  {"id":9,"kind":"Freeze","start_ms":1790000000000,"end_ms":1790000005000,"severity":"Medium","cause":{"Driver":{"core":3}}},
  {"id":9,"kind":"Freeze","start_ms":1790000000000,"end_ms":1790000005000,"severity":"Medium","cause":"GpuDriver"},
  {"id":9,"kind":"Freeze","start_ms":1790000000000,"end_ms":1790000005000,"severity":"Medium","cause":"Throttling"},
  {"id":9,"kind":"Freeze","start_ms":1790000000000,"end_ms":1790000005000,"severity":"Medium","cause":{"App":{"name":"example.exe"}}},
  {"id":9,"kind":"Freeze","start_ms":1790000000000,"end_ms":1790000005000,"severity":"Medium","cause":"Unclear"},
  {"id":9,"kind":"Freeze","start_ms":1790000000000,"end_ms":1790000005000,"severity":"Medium","cause":{"DriverModule":{"module":"example.sys","intermediary":false}}},
  {"id":9,"kind":"Freeze","start_ms":1790000000000,"end_ms":1790000005000,"severity":"Medium","cause":{"Frames":{"app":"example.exe"}}},
  {"id":9,"kind":"Freeze","start_ms":1790000000000,"end_ms":1790000005000,"severity":"Medium","cause":{"BlueScreen":{"code":159}}},
  {"id":9,"kind":"Freeze","start_ms":1790000000000,"end_ms":1790000005000,"severity":"Medium","cause":"Unresponsive"},
  {"id":9,"kind":"Freeze","start_ms":1790000000000,"end_ms":1790000005000,"severity":"Medium","cause":"SleepTransition"},
  {"id":9,"kind":"Freeze","start_ms":1790000000000,"end_ms":1790000005000,"severity":"Medium","cause":{"PowerCut":{"no_dump":true}}},
  {"id":9,"kind":"Freeze","start_ms":1790000000000,"end_ms":1790000005000,"severity":"Medium","cause":"HardwareFault"},
  {"id":9,"kind":"Freeze","start_ms":1790000000000,"end_ms":1790000005000,"severity":"Medium","cause":"NotMeasured"},
  {"id":9,"kind":"Freeze","start_ms":1790000000000,"end_ms":1790000005000,"severity":"Medium","cause":{"Storage":{"disk":null}}},
  {"id":9,"kind":"Freeze","start_ms":1790000000000,"end_ms":1790000005000,"severity":"Medium","cause":{"DiskIo":{"disk":"1 D:","hdd":false,"process":null}}},
  {"id":9,"kind":"Freeze","start_ms":1790000000000,"end_ms":1790000005000,"severity":"Medium","cause":{"Memory":{"process":null}}},
  {"id":9,"kind":"Freeze","start_ms":1790000000000,"end_ms":1790000005000,"severity":"Medium","cause":{"DriverModule":{"module":"example.sys","intermediary":true}}},
  {"id":9,"kind":"Freeze","start_ms":1790000000000,"end_ms":1790000005000,"severity":"Medium","cause":{"PowerCut":{"no_dump":false}}},
] satisfies OccurrenceSummary[];

export const occurrenceSamples = [
  {"kind":"Freeze","start_ms":1790000000000,"end_ms":1790000005000,"severity":"Severe","triggers":[{"Freeze":{"start_ms":1790000000000,"end_ms":1790000005000,"lag_ms":5000.5}},{"Stutter":{"start_ms":1790000000000,"end_ms":1790000004000,"count":6,"worst_ms":220.5}}],"window":{"from_ms":1789999900000,"to_ms":1790000100000,"samples":[{"ts_ms":1790000000000,"cpu_pct":12.5,"dpc_pct":0.5,"isr_pct":0.25,"perf_limit_pct":100.0,"proc_perf_pct":90.0,"mem_total_bytes":17179869184,"mem_avail_bytes":8589934592,"commit_bytes":9000000000,"commit_limit_bytes":20000000000,"hard_faults_per_s":3.5,"gpu_pct":30.0,"probe_max_lag_ms":4.5,"sampler_lag_ms":0.5,"cores":[{"cpu_pct":10.0,"dpc_pct":0.5,"isr_pct":0.25}],"disks":[{"name":"0 C:","active_pct":7.5,"latency_ms":1.5,"queue_len":0.5,"bytes_per_s":1024.0}]}],"tops":[{"ts_ms":1790000000000,"entries":[{"pid":4120,"name":"example.exe","cpu_pct":14.5,"private_bytes":123456789,"read_bps":2048.0,"write_bps":1024.0,"gpu_pct":3.5}]}],"events":[{"ts_ms":1790000000000,"channel":"System","provider":"example-provider","id":129,"record_id":99001,"level":2,"meaning":"StorageReset","data_xml":"<Data Name=\"Example\">1</Data>"},{"ts_ms":1790000000000,"channel":"Application","provider":"example-provider","id":129,"record_id":99001,"level":2,"meaning":"StorageReset","data_xml":"<Data Name=\"Example\">1</Data>"}],"hangs":[{"pid":4120,"name":"example.exe","start_ms":1790000000000,"end_ms":1790000030000,"outcome":"Recovered"},{"pid":4120,"name":"example.exe","start_ms":1790000000000,"end_ms":1790000030000,"outcome":"Closed"},{"pid":4120,"name":"example.exe","start_ms":1790000000000,"end_ms":1790000030000,"outcome":"StillHung"}],"gaps":[{"start_ms":1790000000000,"end_ms":1790000060000,"reason":"Paused"},{"start_ms":1790000000000,"end_ms":1790000060000,"reason":"Suspended"},{"start_ms":1790000000000,"end_ms":1790000060000,"reason":"Stopped"},{"start_ms":1790000000000,"end_ms":1790000060000,"reason":"Crashed"},{"start_ms":1790000000000,"end_ms":1790000060000,"reason":"ClockChanged"},{"start_ms":1790000000000,"end_ms":1790000060000,"reason":"NotSaved"}],"machine":{"logical_cpus":8,"mem_total_bytes":17179869184,"disks":[{"name":"0 C:","media":"Ssd"},{"name":"0 C:","media":"Hdd"},{"name":"0 C:","media":"Unknown"}]}},"suspects":[{"cause":"Unclear","confidence":"Medium","evidence":[{"Event":{"ts_ms":1790000000000,"provider":"example-provider","id":129,"meaning":"StorageReset"}},{"Metric":{"ts_ms":1790000000000,"metric":"DiskLatencyMs","value":450.5,"threshold":100.0,"subject":"0 C:"}}],"next_steps":["UpdateStorageDriver","CheckDiskCable"]}],"facts":[{"at_ms":1790000001000,"kind":{"FreezeStarted":{"duration_ms":5000.5}}},{"at_ms":1790000001000,"kind":"FreezeEnded"}],"sensitive":true,"truncated":true},
  {"kind":"Freeze","start_ms":1790000000000,"end_ms":1790000005000,"severity":"Severe","triggers":[{"Freeze":{"start_ms":1790000000000,"end_ms":1790000005000,"lag_ms":5000.5}},{"Stutter":{"start_ms":1790000000000,"end_ms":1790000004000,"count":6,"worst_ms":220.5}}],"window":{"from_ms":1789999900000,"to_ms":1790000100000,"samples":[{"ts_ms":1790000000000,"cpu_pct":12.5,"dpc_pct":0.5,"isr_pct":0.25,"perf_limit_pct":null,"proc_perf_pct":null,"mem_total_bytes":17179869184,"mem_avail_bytes":8589934592,"commit_bytes":9000000000,"commit_limit_bytes":20000000000,"hard_faults_per_s":null,"gpu_pct":null,"probe_max_lag_ms":4.5,"sampler_lag_ms":0.5,"cores":[{"cpu_pct":10.0,"dpc_pct":0.5,"isr_pct":0.25}],"disks":[{"name":"0 C:","active_pct":7.5,"latency_ms":null,"queue_len":null,"bytes_per_s":null}]}],"tops":[{"ts_ms":1790000000000,"entries":[{"pid":4120,"name":"example.exe","cpu_pct":14.5,"private_bytes":123456789,"read_bps":2048.0,"write_bps":1024.0,"gpu_pct":3.5}]}],"events":[{"ts_ms":1790000000000,"channel":"System","provider":"example-provider","id":129,"record_id":99001,"level":2,"meaning":"StorageReset","data_xml":"<Data Name=\"Example\">1</Data>"},{"ts_ms":1790000000000,"channel":"Application","provider":"example-provider","id":129,"record_id":99001,"level":2,"meaning":"StorageReset","data_xml":"<Data Name=\"Example\">1</Data>"}],"hangs":[{"pid":4120,"name":"example.exe","start_ms":1790000000000,"end_ms":1790000030000,"outcome":"Recovered"},{"pid":4120,"name":"example.exe","start_ms":1790000000000,"end_ms":1790000030000,"outcome":"Closed"},{"pid":4120,"name":"example.exe","start_ms":1790000000000,"end_ms":1790000030000,"outcome":"StillHung"}],"gaps":[{"start_ms":1790000000000,"end_ms":1790000060000,"reason":"Paused"},{"start_ms":1790000000000,"end_ms":1790000060000,"reason":"Suspended"},{"start_ms":1790000000000,"end_ms":1790000060000,"reason":"Stopped"},{"start_ms":1790000000000,"end_ms":1790000060000,"reason":"Crashed"},{"start_ms":1790000000000,"end_ms":1790000060000,"reason":"ClockChanged"},{"start_ms":1790000000000,"end_ms":1790000060000,"reason":"NotSaved"}],"machine":{"logical_cpus":8,"mem_total_bytes":17179869184,"disks":[{"name":"0 C:","media":"Ssd"},{"name":"0 C:","media":"Hdd"},{"name":"0 C:","media":"Unknown"}]}},"suspects":[{"cause":"Unclear","confidence":"Medium","evidence":[{"Event":{"ts_ms":1790000000000,"provider":"example-provider","id":129,"meaning":"StorageReset"}},{"Metric":{"ts_ms":1790000000000,"metric":"DiskLatencyMs","value":450.5,"threshold":100.0,"subject":"0 C:"}}],"next_steps":["UpdateStorageDriver","CheckDiskCable"]}],"facts":[{"at_ms":1790000001000,"kind":{"FreezeStarted":{"duration_ms":5000.5}}},{"at_ms":1790000001000,"kind":"FreezeEnded"}],"sensitive":false,"truncated":false},
] satisfies Occurrence[];

export const occurrenceDtoSamples = [
  {"id":11,"kind":"Freeze","start_ms":1790000000000,"end_ms":1790000005000,"severity":"Severe","triggers":[{"Freeze":{"start_ms":1790000000000,"end_ms":1790000005000,"lag_ms":5000.5}},{"Stutter":{"start_ms":1790000000000,"end_ms":1790000004000,"count":6,"worst_ms":220.5}}],"window":{"from_ms":1789999900000,"to_ms":1790000100000,"samples":[{"ts_ms":1790000000000,"cpu_pct":12.5,"dpc_pct":0.5,"isr_pct":0.25,"perf_limit_pct":100.0,"proc_perf_pct":90.0,"mem_total_bytes":17179869184,"mem_avail_bytes":8589934592,"commit_bytes":9000000000,"commit_limit_bytes":20000000000,"hard_faults_per_s":3.5,"gpu_pct":30.0,"probe_max_lag_ms":4.5,"sampler_lag_ms":0.5,"cores":[{"cpu_pct":10.0,"dpc_pct":0.5,"isr_pct":0.25}],"disks":[{"name":"0 C:","active_pct":7.5,"latency_ms":1.5,"queue_len":0.5,"bytes_per_s":1024.0}]}],"tops":[{"ts_ms":1790000000000,"entries":[{"pid":4120,"name":"example.exe","cpu_pct":14.5,"private_bytes":123456789,"read_bps":2048.0,"write_bps":1024.0,"gpu_pct":3.5}]}],"events":[{"ts_ms":1790000000000,"channel":"System","provider":"example-provider","id":129,"record_id":99001,"level":2,"meaning":"StorageReset","data_xml":"<Data Name=\"Example\">1</Data>"},{"ts_ms":1790000000000,"channel":"Application","provider":"example-provider","id":129,"record_id":99001,"level":2,"meaning":"StorageReset","data_xml":"<Data Name=\"Example\">1</Data>"}],"hangs":[{"pid":4120,"name":"example.exe","start_ms":1790000000000,"end_ms":1790000030000,"outcome":"Recovered"},{"pid":4120,"name":"example.exe","start_ms":1790000000000,"end_ms":1790000030000,"outcome":"Closed"},{"pid":4120,"name":"example.exe","start_ms":1790000000000,"end_ms":1790000030000,"outcome":"StillHung"}],"gaps":[{"start_ms":1790000000000,"end_ms":1790000060000,"reason":"Paused"},{"start_ms":1790000000000,"end_ms":1790000060000,"reason":"Suspended"},{"start_ms":1790000000000,"end_ms":1790000060000,"reason":"Stopped"},{"start_ms":1790000000000,"end_ms":1790000060000,"reason":"Crashed"},{"start_ms":1790000000000,"end_ms":1790000060000,"reason":"ClockChanged"},{"start_ms":1790000000000,"end_ms":1790000060000,"reason":"NotSaved"}],"machine":{"logical_cpus":8,"mem_total_bytes":17179869184,"disks":[{"name":"0 C:","media":"Ssd"},{"name":"0 C:","media":"Hdd"},{"name":"0 C:","media":"Unknown"}]}},"suspects":[{"cause":"Unclear","confidence":"Medium","evidence":[{"Event":{"ts_ms":1790000000000,"provider":"example-provider","id":129,"meaning":"StorageReset"}},{"Metric":{"ts_ms":1790000000000,"metric":"DiskLatencyMs","value":450.5,"threshold":100.0,"subject":"0 C:"}}],"next_steps":["UpdateStorageDriver","CheckDiskCable"]}],"facts":[{"at_ms":1790000001000,"kind":{"FreezeStarted":{"duration_ms":5000.5}}},{"at_ms":1790000001000,"kind":"FreezeEnded"}],"sensitive":true,"truncated":true,"chart":{"t":[1790000000.0,1790000001.0],"cpu":[12.5,null],"mem_used_pct":[61.5,null],"disk_active_max":[7.5,null],"disk_latency_max":[1.5,null],"gpu":[30.0,null],"probe_lag":[0.5,null],"state":["calm","suspended"]},"capture":{"id":4,"start_ms":1790000000000,"end_ms":1790000060000,"suspects":[{"cause":"Unclear","confidence":"Medium","evidence":[{"Event":{"ts_ms":1790000000000,"provider":"example-provider","id":129,"meaning":"StorageReset"}},{"Metric":{"ts_ms":1790000000000,"metric":"DiskLatencyMs","value":450.5,"threshold":100.0,"subject":"0 C:"}}],"next_steps":["UpdateStorageDriver","CheckDiskCable"]}]},"tests":[{"id":5,"kind":"MemoryQuick","subject":{"Disk":"0 C:"},"started_ms":1790000000000,"ended_ms":1790000600000,"outcome":"Clean"},{"id":5,"kind":"DiskSurface","subject":{"Disk":"0 C:"},"started_ms":1790000000000,"ended_ms":1790000600000,"outcome":"Problems"},{"id":5,"kind":"VolumeCheck","subject":{"Disk":"0 C:"},"started_ms":1790000000000,"ended_ms":1790000600000,"outcome":"Interrupted"},{"id":5,"kind":"DiskBenchmark","subject":{"Disk":"0 C:"},"started_ms":1790000000000,"ended_ms":1790000600000,"outcome":"Failed"}]},
  {"id":11,"kind":"Freeze","start_ms":1790000000000,"end_ms":1790000005000,"severity":"Severe","triggers":[{"Freeze":{"start_ms":1790000000000,"end_ms":1790000005000,"lag_ms":5000.5}},{"Stutter":{"start_ms":1790000000000,"end_ms":1790000004000,"count":6,"worst_ms":220.5}}],"window":{"from_ms":1789999900000,"to_ms":1790000100000,"samples":[{"ts_ms":1790000000000,"cpu_pct":12.5,"dpc_pct":0.5,"isr_pct":0.25,"perf_limit_pct":null,"proc_perf_pct":null,"mem_total_bytes":17179869184,"mem_avail_bytes":8589934592,"commit_bytes":9000000000,"commit_limit_bytes":20000000000,"hard_faults_per_s":null,"gpu_pct":null,"probe_max_lag_ms":4.5,"sampler_lag_ms":0.5,"cores":[{"cpu_pct":10.0,"dpc_pct":0.5,"isr_pct":0.25}],"disks":[{"name":"0 C:","active_pct":7.5,"latency_ms":null,"queue_len":null,"bytes_per_s":null}]}],"tops":[{"ts_ms":1790000000000,"entries":[{"pid":4120,"name":"example.exe","cpu_pct":14.5,"private_bytes":123456789,"read_bps":2048.0,"write_bps":1024.0,"gpu_pct":3.5}]}],"events":[{"ts_ms":1790000000000,"channel":"System","provider":"example-provider","id":129,"record_id":99001,"level":2,"meaning":"StorageReset","data_xml":"<Data Name=\"Example\">1</Data>"},{"ts_ms":1790000000000,"channel":"Application","provider":"example-provider","id":129,"record_id":99001,"level":2,"meaning":"StorageReset","data_xml":"<Data Name=\"Example\">1</Data>"}],"hangs":[{"pid":4120,"name":"example.exe","start_ms":1790000000000,"end_ms":1790000030000,"outcome":"Recovered"},{"pid":4120,"name":"example.exe","start_ms":1790000000000,"end_ms":1790000030000,"outcome":"Closed"},{"pid":4120,"name":"example.exe","start_ms":1790000000000,"end_ms":1790000030000,"outcome":"StillHung"}],"gaps":[{"start_ms":1790000000000,"end_ms":1790000060000,"reason":"Paused"},{"start_ms":1790000000000,"end_ms":1790000060000,"reason":"Suspended"},{"start_ms":1790000000000,"end_ms":1790000060000,"reason":"Stopped"},{"start_ms":1790000000000,"end_ms":1790000060000,"reason":"Crashed"},{"start_ms":1790000000000,"end_ms":1790000060000,"reason":"ClockChanged"},{"start_ms":1790000000000,"end_ms":1790000060000,"reason":"NotSaved"}],"machine":{"logical_cpus":8,"mem_total_bytes":17179869184,"disks":[{"name":"0 C:","media":"Ssd"},{"name":"0 C:","media":"Hdd"},{"name":"0 C:","media":"Unknown"}]}},"suspects":[{"cause":"Unclear","confidence":"Medium","evidence":[{"Event":{"ts_ms":1790000000000,"provider":"example-provider","id":129,"meaning":"StorageReset"}},{"Metric":{"ts_ms":1790000000000,"metric":"DiskLatencyMs","value":450.5,"threshold":100.0,"subject":"0 C:"}}],"next_steps":["UpdateStorageDriver","CheckDiskCable"]}],"facts":[{"at_ms":1790000001000,"kind":{"FreezeStarted":{"duration_ms":5000.5}}},{"at_ms":1790000001000,"kind":"FreezeEnded"}],"sensitive":false,"truncated":false,"chart":{"t":[1790000000.0,1790000001.0],"cpu":[12.5,null],"mem_used_pct":[61.5,null],"disk_active_max":[7.5,null],"disk_latency_max":[1.5,null],"gpu":[30.0,null],"probe_lag":[0.5,null],"state":["calm","suspended"]},"capture":null,"tests":[]},
] satisfies OccurrenceDto[];

export const episodeKindSamples = [
  "MemoryHigh",
  {"DiskBusy":{"disk":"0 C:"}},
  "CpuSaturated",
  {"DpcIsrHint":{"core":2}},
  {"DiskLatencyHint":{"disk":"0 C:"}},
] satisfies EpisodeKind[];

export const culpritSamples = [
  {"pid":4120,"name":"example.exe","value":1500000000.0},
] satisfies Culprit[];

export const episodeSamples = [
  {"kind":"MemoryHigh","start_ms":1790000000000,"end_ms":1790000600000,"peak":97.5,"culprit":{"pid":4120,"name":"example.exe","value":1500000000.0},"truncated":true},
  {"kind":"MemoryHigh","start_ms":1790000000000,"end_ms":1790000600000,"peak":97.5,"culprit":null,"truncated":false},
  {"kind":{"DiskBusy":{"disk":"0 C:"}},"start_ms":1790000000000,"end_ms":1790000600000,"peak":97.5,"culprit":{"pid":4120,"name":"example.exe","value":1500000000.0},"truncated":true},
  {"kind":{"DiskBusy":{"disk":"0 C:"}},"start_ms":1790000000000,"end_ms":1790000600000,"peak":97.5,"culprit":null,"truncated":false},
  {"kind":"CpuSaturated","start_ms":1790000000000,"end_ms":1790000600000,"peak":97.5,"culprit":{"pid":4120,"name":"example.exe","value":1500000000.0},"truncated":true},
  {"kind":"CpuSaturated","start_ms":1790000000000,"end_ms":1790000600000,"peak":97.5,"culprit":null,"truncated":false},
  {"kind":{"DpcIsrHint":{"core":2}},"start_ms":1790000000000,"end_ms":1790000600000,"peak":97.5,"culprit":{"pid":4120,"name":"example.exe","value":1500000000.0},"truncated":true},
  {"kind":{"DpcIsrHint":{"core":2}},"start_ms":1790000000000,"end_ms":1790000600000,"peak":97.5,"culprit":null,"truncated":false},
  {"kind":{"DiskLatencyHint":{"disk":"0 C:"}},"start_ms":1790000000000,"end_ms":1790000600000,"peak":97.5,"culprit":{"pid":4120,"name":"example.exe","value":1500000000.0},"truncated":true},
  {"kind":{"DiskLatencyHint":{"disk":"0 C:"}},"start_ms":1790000000000,"end_ms":1790000600000,"peak":97.5,"culprit":null,"truncated":false},
] satisfies Episode[];

export const episodeDetailDtoSamples = [
  {"history":"ready","resolution":"seconds","step_ms":1000,"from_ms":1789999000000,"to_ms":1790000900000,"chart":{"t":[1790000000.0,1790000001.0],"cpu":[12.5,null],"mem_used_pct":[61.5,null],"disk_active_max":[7.5,null],"disk_latency_max":[1.5,null],"gpu":[30.0,null],"probe_lag":[0.5,null],"state":["calm","suspended"]},"programs":[{"pid":4120,"name":"example.exe","cpu_pct":14.5,"private_bytes":123456789,"read_bps":2048.0,"write_bps":1024.0,"gpu_pct":3.5}],"occurrences":[{"id":1,"kind":"Freeze","start_ms":1790000000000,"end_ms":1790000005000,"severity":"Light","cause":"Unclear"},{"id":2,"kind":"Stutter","start_ms":1790000000000,"end_ms":1790000005000,"severity":"Medium","cause":"Unclear"}]},
  {"history":"ready","resolution":"minutes","step_ms":1000,"from_ms":1789999000000,"to_ms":1790000900000,"chart":{"t":[1790000000.0,1790000001.0],"cpu":[12.5,null],"mem_used_pct":[61.5,null],"disk_active_max":[7.5,null],"disk_latency_max":[1.5,null],"gpu":[30.0,null],"probe_lag":[0.5,null],"state":["calm","suspended"]},"programs":[{"pid":4120,"name":"example.exe","cpu_pct":14.5,"private_bytes":123456789,"read_bps":2048.0,"write_bps":1024.0,"gpu_pct":3.5}],"occurrences":[{"id":1,"kind":"Freeze","start_ms":1790000000000,"end_ms":1790000005000,"severity":"Light","cause":"Unclear"},{"id":2,"kind":"Stutter","start_ms":1790000000000,"end_ms":1790000005000,"severity":"Medium","cause":"Unclear"}]},
  {"history":"empty","resolution":"seconds","step_ms":1000,"from_ms":1789999000000,"to_ms":1790000900000,"chart":{"t":[1790000000.0,1790000001.0],"cpu":[12.5,null],"mem_used_pct":[61.5,null],"disk_active_max":[7.5,null],"disk_latency_max":[1.5,null],"gpu":[30.0,null],"probe_lag":[0.5,null],"state":["calm","suspended"]},"programs":[{"pid":4120,"name":"example.exe","cpu_pct":14.5,"private_bytes":123456789,"read_bps":2048.0,"write_bps":1024.0,"gpu_pct":3.5}],"occurrences":[{"id":1,"kind":"Freeze","start_ms":1790000000000,"end_ms":1790000005000,"severity":"Light","cause":"Unclear"},{"id":2,"kind":"Stutter","start_ms":1790000000000,"end_ms":1790000005000,"severity":"Medium","cause":"Unclear"}]},
] satisfies EpisodeDetailDto[];

export const marksDtoSamples = [
  {"history":"ready","occurrences":[{"id":1,"kind":"Freeze","start_ms":1790000000000,"end_ms":1790000005000,"severity":"Light","cause":"Unclear"},{"id":2,"kind":"Stutter","start_ms":1790000000000,"end_ms":1790000005000,"severity":"Medium","cause":"Unclear"}],"episodes":[{"kind":"MemoryHigh","start_ms":1790000000000,"end_ms":1790000600000,"peak":97.5,"culprit":{"pid":4120,"name":"example.exe","value":1500000000.0},"truncated":true},{"kind":"MemoryHigh","start_ms":1790000000000,"end_ms":1790000600000,"peak":97.5,"culprit":null,"truncated":false}],"by_test":[true,false],"gaps":[{"start_ms":1790000000000,"end_ms":1790000060000,"reason":"Paused"},{"start_ms":1790000000000,"end_ms":1790000060000,"reason":"Suspended"},{"start_ms":1790000000000,"end_ms":1790000060000,"reason":"Stopped"},{"start_ms":1790000000000,"end_ms":1790000060000,"reason":"Crashed"},{"start_ms":1790000000000,"end_ms":1790000060000,"reason":"ClockChanged"},{"start_ms":1790000000000,"end_ms":1790000060000,"reason":"NotSaved"}],"tests":[{"id":5,"kind":"MemoryQuick","subject":{"Disk":"0 C:"},"started_ms":1790000000000,"ended_ms":1790000600000,"outcome":"Clean"},{"id":5,"kind":"DiskSurface","subject":{"Disk":"0 C:"},"started_ms":1790000000000,"ended_ms":1790000600000,"outcome":"Problems"},{"id":5,"kind":"VolumeCheck","subject":{"Disk":"0 C:"},"started_ms":1790000000000,"ended_ms":1790000600000,"outcome":"Interrupted"},{"id":5,"kind":"DiskBenchmark","subject":{"Disk":"0 C:"},"started_ms":1790000000000,"ended_ms":1790000600000,"outcome":"Failed"}]},
  {"history":"ready","occurrences":[{"id":1,"kind":"Freeze","start_ms":1790000000000,"end_ms":1790000005000,"severity":"Light","cause":"Unclear"},{"id":2,"kind":"Stutter","start_ms":1790000000000,"end_ms":1790000005000,"severity":"Medium","cause":"Unclear"}],"episodes":[{"kind":"MemoryHigh","start_ms":1790000000000,"end_ms":1790000600000,"peak":97.5,"culprit":{"pid":4120,"name":"example.exe","value":1500000000.0},"truncated":true},{"kind":"MemoryHigh","start_ms":1790000000000,"end_ms":1790000600000,"peak":97.5,"culprit":null,"truncated":false}],"by_test":[true,false],"gaps":[{"start_ms":1790000000000,"end_ms":1790000060000,"reason":"Paused"},{"start_ms":1790000000000,"end_ms":1790000060000,"reason":"Suspended"},{"start_ms":1790000000000,"end_ms":1790000060000,"reason":"Stopped"},{"start_ms":1790000000000,"end_ms":1790000060000,"reason":"Crashed"},{"start_ms":1790000000000,"end_ms":1790000060000,"reason":"ClockChanged"},{"start_ms":1790000000000,"end_ms":1790000060000,"reason":"NotSaved"}],"tests":[{"id":5,"kind":"MemoryQuick","subject":{"Disk":"0 C:"},"started_ms":1790000000000,"ended_ms":1790000600000,"outcome":"Clean"},{"id":5,"kind":"DiskSurface","subject":{"Disk":"0 C:"},"started_ms":1790000000000,"ended_ms":1790000600000,"outcome":"Problems"},{"id":5,"kind":"VolumeCheck","subject":{"Disk":"0 C:"},"started_ms":1790000000000,"ended_ms":1790000600000,"outcome":"Interrupted"},{"id":5,"kind":"DiskBenchmark","subject":{"Disk":"0 C:"},"started_ms":1790000000000,"ended_ms":1790000600000,"outcome":"Failed"}]},
  {"history":"empty","occurrences":[{"id":1,"kind":"Freeze","start_ms":1790000000000,"end_ms":1790000005000,"severity":"Light","cause":"Unclear"},{"id":2,"kind":"Stutter","start_ms":1790000000000,"end_ms":1790000005000,"severity":"Medium","cause":"Unclear"}],"episodes":[{"kind":"MemoryHigh","start_ms":1790000000000,"end_ms":1790000600000,"peak":97.5,"culprit":{"pid":4120,"name":"example.exe","value":1500000000.0},"truncated":true},{"kind":"MemoryHigh","start_ms":1790000000000,"end_ms":1790000600000,"peak":97.5,"culprit":null,"truncated":false}],"by_test":[true,false],"gaps":[{"start_ms":1790000000000,"end_ms":1790000060000,"reason":"Paused"},{"start_ms":1790000000000,"end_ms":1790000060000,"reason":"Suspended"},{"start_ms":1790000000000,"end_ms":1790000060000,"reason":"Stopped"},{"start_ms":1790000000000,"end_ms":1790000060000,"reason":"Crashed"},{"start_ms":1790000000000,"end_ms":1790000060000,"reason":"ClockChanged"},{"start_ms":1790000000000,"end_ms":1790000060000,"reason":"NotSaved"}],"tests":[{"id":5,"kind":"MemoryQuick","subject":{"Disk":"0 C:"},"started_ms":1790000000000,"ended_ms":1790000600000,"outcome":"Clean"},{"id":5,"kind":"DiskSurface","subject":{"Disk":"0 C:"},"started_ms":1790000000000,"ended_ms":1790000600000,"outcome":"Problems"},{"id":5,"kind":"VolumeCheck","subject":{"Disk":"0 C:"},"started_ms":1790000000000,"ended_ms":1790000600000,"outcome":"Interrupted"},{"id":5,"kind":"DiskBenchmark","subject":{"Disk":"0 C:"},"started_ms":1790000000000,"ended_ms":1790000600000,"outcome":"Failed"}]},
] satisfies MarksDto[];

export const trayStateSamples = [
  "normal",
  "attention",
  "problem",
] satisfies TrayState[];

export const agentStatusSamples = [
  {"paused":true,"paused_until_ms":1790000300000,"sensitive_until_ms":1790000600000,"state":"attention","started_at_ms":1789000000000,"last_sample_ms":1790000000000,"last_error":"pipe closed","suggest_capture":true,"cpu_pct":1.5,"suspended":true,"monitor":{"state":"no_reading","failing":[{"source":"counters","since_ms":1789999900000,"error":"counter query failed"}],"no_reading_since_ms":1790000000000,"resumed_ms":1789999000000,"health_attempt_ms":1790000200000,"health_ok_ms":1790000100000,"health_unread":["TwoAntivirus","DiskSmart"]}},
  {"paused":false,"paused_until_ms":null,"sensitive_until_ms":null,"state":"normal","started_at_ms":1789000000000,"last_sample_ms":null,"last_error":null,"suggest_capture":false,"cpu_pct":null,"suspended":false,"monitor":{"state":"watching","failing":[],"no_reading_since_ms":null,"resumed_ms":null,"health_attempt_ms":null,"health_ok_ms":null,"health_unread":[]}},
] satisfies AgentStatus[];

export const monitorStatusSamples = [
  {"state":"no_reading","failing":[{"source":"counters","since_ms":1789999900000,"error":"counter query failed"}],"no_reading_since_ms":1790000000000,"resumed_ms":1789999000000,"health_attempt_ms":1790000200000,"health_ok_ms":1790000100000,"health_unread":["TwoAntivirus","DiskSmart"]},
  {"state":"watching","failing":[],"no_reading_since_ms":null,"resumed_ms":null,"health_attempt_ms":null,"health_ok_ms":null,"health_unread":[]},
] satisfies MonitorStatus[];

export const monitorStateSamples = [
  "watching",
  "degraded",
  "no_reading",
] satisfies MonitorState[];

export const monitorSourceSamples = [
  "memory",
  "cpu",
  "counters",
  "processes",
  "timer",
] satisfies MonitorSource[];

export const sourceFailureSamples = [
  {"source":"counters","since_ms":1789999900000,"error":"counter query failed"},
] satisfies SourceFailure[];

export const agentLinkSamples = [
  {"link":"running","paused":true,"paused_until_ms":1790000300000,"sensitive_until_ms":1790000600000,"state":"attention","started_at_ms":1789000000000,"last_sample_ms":1790000000000,"last_error":"pipe closed","suggest_capture":true,"cpu_pct":1.5,"suspended":true,"monitor":{"state":"no_reading","failing":[{"source":"counters","since_ms":1789999900000,"error":"counter query failed"}],"no_reading_since_ms":1790000000000,"resumed_ms":1789999000000,"health_attempt_ms":1790000200000,"health_ok_ms":1790000100000,"health_unread":["TwoAntivirus","DiskSmart"]}},
  {"link":"stopped"},
  {"link":"mismatch"},
  {"link":"running","paused":false,"paused_until_ms":null,"sensitive_until_ms":null,"state":"normal","started_at_ms":1789000000000,"last_sample_ms":null,"last_error":null,"suggest_capture":false,"cpu_pct":null,"suspended":false,"monitor":{"state":"watching","failing":[],"no_reading_since_ms":null,"resumed_ms":null,"health_attempt_ms":null,"health_ok_ms":null,"health_unread":[]}},
] satisfies AgentLink[];

export const retentionSamples = [
  {"samples_hours":48,"minutes_days":30,"records_days":365},
] satisfies Retention[];

export const languagePrefSamples = [
  "System",
  "En",
  "PtBr",
] satisfies LanguagePref[];

export const themePrefSamples = [
  "System",
  "Light",
  "Dark",
] satisfies ThemePref[];

export const colorThemeSamples = [
  "Red",
  "Graphite",
  "Slate",
  "Indigo",
] satisfies ColorTheme[];

export const detailLevelSamples = [
  "Simple",
  "Detailed",
  "Technical",
] satisfies DetailLevel[];

export const settingsSamples = [
  {"retention":{"samples_hours":48,"minutes_days":30,"records_days":365},"language":"System","theme":"System","color_theme":"Red","detail_level":"Simple"},
  {"retention":{"samples_hours":48,"minutes_days":30,"records_days":365},"language":"En","theme":"Light","color_theme":"Graphite","detail_level":"Detailed"},
  {"retention":{"samples_hours":48,"minutes_days":30,"records_days":365},"language":"PtBr","theme":"Dark","color_theme":"Slate","detail_level":"Technical"},
  {"retention":{"samples_hours":48,"minutes_days":30,"records_days":365},"language":"System","theme":"System","color_theme":"Indigo","detail_level":"Simple"},
  {"retention":{"samples_hours":48,"minutes_days":30,"records_days":365},"language":"System","theme":"Light","color_theme":"Slate","detail_level":"Simple"},
  {"retention":{"samples_hours":48,"minutes_days":30,"records_days":365},"language":"System","theme":"Dark","color_theme":"Graphite","detail_level":"Simple"},
  {"retention":{"samples_hours":48,"minutes_days":30,"records_days":365},"language":"System","theme":"System","color_theme":"Red","detail_level":"Simple"},
  {"retention":{"samples_hours":48,"minutes_days":30,"records_days":365},"language":"System","theme":"System","color_theme":"Graphite","detail_level":"Simple"},
  {"retention":{"samples_hours":48,"minutes_days":30,"records_days":365},"language":"System","theme":"System","color_theme":"Slate","detail_level":"Simple"},
  {"retention":{"samples_hours":48,"minutes_days":30,"records_days":365},"language":"System","theme":"System","color_theme":"Indigo","detail_level":"Simple"},
  {"retention":{"samples_hours":48,"minutes_days":30,"records_days":365},"language":"System","theme":"System","color_theme":"Red","detail_level":"Simple"},
  {"retention":{"samples_hours":48,"minutes_days":30,"records_days":365},"language":"System","theme":"System","color_theme":"Red","detail_level":"Detailed"},
  {"retention":{"samples_hours":48,"minutes_days":30,"records_days":365},"language":"System","theme":"System","color_theme":"Red","detail_level":"Technical"},
] satisfies Settings[];

export const settingsDtoSamples = [
  {"settings":{"retention":{"samples_hours":48,"minutes_days":30,"records_days":365},"language":"System","theme":"System","color_theme":"Red","detail_level":"Simple"},"autostart":true,"editable":true},
  {"settings":{"retention":{"samples_hours":48,"minutes_days":30,"records_days":365},"language":"En","theme":"Light","color_theme":"Graphite","detail_level":"Detailed"},"autostart":null,"editable":false},
  {"settings":{"retention":{"samples_hours":48,"minutes_days":30,"records_days":365},"language":"System","theme":"System","color_theme":"Red","detail_level":"Simple"},"autostart":false,"editable":false},
] satisfies SettingsDto[];

export const nowFrameSamples = [
  {"sample":{"ts_ms":1790000000000,"cpu_pct":12.5,"dpc_pct":0.5,"isr_pct":0.25,"perf_limit_pct":100.0,"proc_perf_pct":90.0,"mem_total_bytes":17179869184,"mem_avail_bytes":8589934592,"commit_bytes":9000000000,"commit_limit_bytes":20000000000,"hard_faults_per_s":3.5,"gpu_pct":30.0,"probe_max_lag_ms":4.5,"sampler_lag_ms":0.5,"cores":[{"cpu_pct":10.0,"dpc_pct":0.5,"isr_pct":0.25}],"disks":[{"name":"0 C:","active_pct":7.5,"latency_ms":1.5,"queue_len":0.5,"bytes_per_s":1024.0}]},"top":{"ts_ms":1790000000000,"entries":[{"pid":4120,"name":"example.exe","cpu_pct":14.5,"private_bytes":123456789,"read_bps":2048.0,"write_bps":1024.0,"gpu_pct":3.5}]},"programs":{"ts_ms":1790000000000,"entries":[{"pid":4120,"name":"example.exe","cpu_pct":14.5,"private_bytes":123456789,"read_bps":2048.0,"write_bps":1024.0,"gpu_pct":3.5}]},"hung":[{"pid":4120,"name":"example.exe"}],"latest":{"id":1,"kind":"Freeze","start_ms":1790000000000,"end_ms":1790000005000,"severity":"Light","cause":"Unclear"},"pressure":["MemoryHigh",{"DiskBusy":{"disk":"0 C:"}},"CpuSaturated",{"DpcIsrHint":{"core":2}},{"DiskLatencyHint":{"disk":"0 C:"}}]},
  {"sample":null,"top":null,"programs":null,"hung":[{"pid":4120,"name":"example.exe"}],"latest":null,"pressure":["MemoryHigh",{"DiskBusy":{"disk":"0 C:"}},"CpuSaturated",{"DpcIsrHint":{"core":2}},{"DiskLatencyHint":{"disk":"0 C:"}}]},
  {"sample":null,"top":null,"programs":null,"hung":[],"latest":null,"pressure":[]},
] satisfies NowFrame[];

export const lanesSamples = [
  {"cpu":"calm","memory":"busy","disk":"pressure","gpu":"suspended"},
] satisfies Lanes[];

export const livePointSamples = [
  {"reading":{"ts_ms":1790000000000,"cpu_pct":12.5,"mem_used_pct":61.5,"disk_active_pct":7.5,"disk_latency_ms":1.5,"gpu_pct":30.0,"probe_lag_ms":0.5},"state":"busy","lanes":{"cpu":"calm","memory":"busy","disk":"pressure","gpu":"suspended"}},
] satisfies LivePoint[];

export const liveMessageSamples = [
  {"kind":"recent","points":[{"reading":{"ts_ms":1790000000000,"cpu_pct":12.5,"mem_used_pct":61.5,"disk_active_pct":7.5,"disk_latency_ms":1.5,"gpu_pct":30.0,"probe_lag_ms":0.5},"state":"busy","lanes":{"cpu":"calm","memory":"busy","disk":"pressure","gpu":"suspended"}}]},
  {"kind":"now","point":{"reading":{"ts_ms":1790000000000,"cpu_pct":12.5,"mem_used_pct":61.5,"disk_active_pct":7.5,"disk_latency_ms":1.5,"gpu_pct":30.0,"probe_lag_ms":0.5},"state":"busy","lanes":{"cpu":"calm","memory":"busy","disk":"pressure","gpu":"suspended"}},"frame":{"sample":{"ts_ms":1790000000000,"cpu_pct":12.5,"dpc_pct":0.5,"isr_pct":0.25,"perf_limit_pct":100.0,"proc_perf_pct":90.0,"mem_total_bytes":17179869184,"mem_avail_bytes":8589934592,"commit_bytes":9000000000,"commit_limit_bytes":20000000000,"hard_faults_per_s":3.5,"gpu_pct":30.0,"probe_max_lag_ms":4.5,"sampler_lag_ms":0.5,"cores":[{"cpu_pct":10.0,"dpc_pct":0.5,"isr_pct":0.25}],"disks":[{"name":"0 C:","active_pct":7.5,"latency_ms":1.5,"queue_len":0.5,"bytes_per_s":1024.0}]},"top":{"ts_ms":1790000000000,"entries":[{"pid":4120,"name":"example.exe","cpu_pct":14.5,"private_bytes":123456789,"read_bps":2048.0,"write_bps":1024.0,"gpu_pct":3.5}]},"programs":{"ts_ms":1790000000000,"entries":[{"pid":4120,"name":"example.exe","cpu_pct":14.5,"private_bytes":123456789,"read_bps":2048.0,"write_bps":1024.0,"gpu_pct":3.5}]},"hung":[{"pid":4120,"name":"example.exe"}],"latest":{"id":1,"kind":"Freeze","start_ms":1790000000000,"end_ms":1790000005000,"severity":"Light","cause":"Unclear"},"pressure":["MemoryHigh",{"DiskBusy":{"disk":"0 C:"}},"CpuSaturated",{"DpcIsrHint":{"core":2}},{"DiskLatencyHint":{"disk":"0 C:"}}]}},
  {"kind":"stopped"},
  {"kind":"now","point":null,"frame":{"sample":null,"top":null,"programs":null,"hung":[{"pid":4120,"name":"example.exe"}],"latest":null,"pressure":["MemoryHigh",{"DiskBusy":{"disk":"0 C:"}},"CpuSaturated",{"DpcIsrHint":{"core":2}},{"DiskLatencyHint":{"disk":"0 C:"}}]}},
] satisfies LiveMessage[];

export const uiErrorSamples = [
  {"kind":"agent_stopped","message":"the watcher is not running"},
  {"kind":"mismatch","message":"the pipe belongs to another program, or to a watcher of another version"},
  {"kind":"refused","message":"the watcher refused: not allowed"},
  {"kind":"start_timeout","message":"the watcher did not answer within 3 s"},
  {"kind":"store","message":"history: no history database yet"},
  {"kind":"ipc","message":"pipe: another agent already owns the pipe"},
  {"kind":"failed","message":"something failed"},
  {"kind":"declined","message":"the permission prompt was declined"},
  {"kind":"changed_since","message":"a setting now holds a value that Vestigium did not write"},
] satisfies UiError[];

export const uiErrorKindSamples = [
  "agent_stopped",
  "mismatch",
  "refused",
  "start_timeout",
  "store",
  "ipc",
  "failed",
  "declined",
  "changed_since",
] satisfies UiErrorKind[];

export const routineKindSamples = [
  "Dpc",
  "Isr",
] satisfies RoutineKind[];

export const routineStatsSamples = [
  {"count":120,"total_us":54000,"max_us":900,"buckets":[1,1,1,1,1,1,1,1,1,1,1,1,1]},
] satisfies RoutineStats[];

export const driverLatencySamples = [
  {"module":"example.sys","dpc":{"count":120,"total_us":54000,"max_us":900,"buckets":[1,1,1,1,1,1,1,1,1,1,1,1,1]},"isr":{"count":120,"total_us":54000,"max_us":900,"buckets":[1,1,1,1,1,1,1,1,1,1,1,1,1]}},
] satisfies DriverLatency[];

export const longRoutineSamples = [
  {"ts_ms":1790000000000,"core":1,"module":"example.sys","kind":"Dpc","us":1500},
  {"ts_ms":1790000000000,"core":1,"module":"example.sys","kind":"Isr","us":1500},
] satisfies LongRoutine[];

export const appFramesSamples = [
  {"name":"example.exe","pid":4120,"frames":600,"median_ms":8.5,"p99_ms":30.5,"worst_ms":80.5,"hitches":12},
] satisfies AppFrames[];

export const frameHitchSamples = [
  {"ts_ms":1790000000000,"app":"example.exe","pid":4120,"frame_ms":70.5,"median_ms":8.5},
] satisfies FrameHitch[];

export const probeHiccupSamples = [
  {"ts_ms":1790000000000,"lag_ms":250.5},
] satisfies ProbeHiccup[];

export const captureSourceSamples = [
  "Ok",
  {"Unavailable":"no access"},
  {"Partial":{"covered_secs":40,"reason":"stopped early"}},
] satisfies CaptureSource[];

export const captureSummarySamples = [
  {"id":4,"start_ms":1790000000000,"end_ms":1790000060000},
] satisfies CaptureSummary[];

export const captureDtoSamples = [
  {"id":4,"start_ms":1790000000000,"end_ms":1790000060000,"kernel":"Ok","frames":{"Partial":{"covered_secs":30,"reason":"stopped early"}},"lost_events":3,"drivers":[{"module":"example.sys","dpc":{"count":120,"total_us":54000,"max_us":900,"buckets":[1,1,1,1,1,1,1,1,1,1,1,1,1]},"isr":{"count":120,"total_us":54000,"max_us":900,"buckets":[1,1,1,1,1,1,1,1,1,1,1,1,1]}}],"longest":[{"ts_ms":1790000000000,"core":1,"module":"example.sys","kind":"Dpc","us":1500},{"ts_ms":1790000000000,"core":1,"module":"example.sys","kind":"Isr","us":1500}],"apps":[{"name":"example.exe","pid":4120,"frames":600,"median_ms":8.5,"p99_ms":30.5,"worst_ms":80.5,"hitches":12}],"hitches":[{"ts_ms":1790000000000,"app":"example.exe","pid":4120,"frame_ms":70.5,"median_ms":8.5}],"hiccups":[{"ts_ms":1790000000000,"lag_ms":250.5}],"findings":[{"cause":"Unclear","confidence":"Medium","evidence":[{"Event":{"ts_ms":1790000000000,"provider":"example-provider","id":129,"meaning":"StorageReset"}},{"Metric":{"ts_ms":1790000000000,"metric":"DiskLatencyMs","value":450.5,"threshold":100.0,"subject":"0 C:"}}],"next_steps":["UpdateStorageDriver","CheckDiskCable"]}]},
  {"id":4,"start_ms":1790000000000,"end_ms":1790000060000,"kernel":{"Unavailable":"no access"},"frames":{"Partial":{"covered_secs":30,"reason":"stopped early"}},"lost_events":3,"drivers":[{"module":"example.sys","dpc":{"count":120,"total_us":54000,"max_us":900,"buckets":[1,1,1,1,1,1,1,1,1,1,1,1,1]},"isr":{"count":120,"total_us":54000,"max_us":900,"buckets":[1,1,1,1,1,1,1,1,1,1,1,1,1]}}],"longest":[{"ts_ms":1790000000000,"core":1,"module":"example.sys","kind":"Dpc","us":1500},{"ts_ms":1790000000000,"core":1,"module":"example.sys","kind":"Isr","us":1500}],"apps":[{"name":"example.exe","pid":4120,"frames":600,"median_ms":8.5,"p99_ms":30.5,"worst_ms":80.5,"hitches":12}],"hitches":[{"ts_ms":1790000000000,"app":"example.exe","pid":4120,"frame_ms":70.5,"median_ms":8.5}],"hiccups":[{"ts_ms":1790000000000,"lag_ms":250.5}],"findings":[{"cause":"Unclear","confidence":"Medium","evidence":[{"Event":{"ts_ms":1790000000000,"provider":"example-provider","id":129,"meaning":"StorageReset"}},{"Metric":{"ts_ms":1790000000000,"metric":"DiskLatencyMs","value":450.5,"threshold":100.0,"subject":"0 C:"}}],"next_steps":["UpdateStorageDriver","CheckDiskCable"]}]},
  {"id":4,"start_ms":1790000000000,"end_ms":1790000060000,"kernel":{"Partial":{"covered_secs":40,"reason":"stopped early"}},"frames":{"Partial":{"covered_secs":30,"reason":"stopped early"}},"lost_events":3,"drivers":[{"module":"example.sys","dpc":{"count":120,"total_us":54000,"max_us":900,"buckets":[1,1,1,1,1,1,1,1,1,1,1,1,1]},"isr":{"count":120,"total_us":54000,"max_us":900,"buckets":[1,1,1,1,1,1,1,1,1,1,1,1,1]}}],"longest":[{"ts_ms":1790000000000,"core":1,"module":"example.sys","kind":"Dpc","us":1500},{"ts_ms":1790000000000,"core":1,"module":"example.sys","kind":"Isr","us":1500}],"apps":[{"name":"example.exe","pid":4120,"frames":600,"median_ms":8.5,"p99_ms":30.5,"worst_ms":80.5,"hitches":12}],"hitches":[{"ts_ms":1790000000000,"app":"example.exe","pid":4120,"frame_ms":70.5,"median_ms":8.5}],"hiccups":[{"ts_ms":1790000000000,"lag_ms":250.5}],"findings":[{"cause":"Unclear","confidence":"Medium","evidence":[{"Event":{"ts_ms":1790000000000,"provider":"example-provider","id":129,"meaning":"StorageReset"}},{"Metric":{"ts_ms":1790000000000,"metric":"DiskLatencyMs","value":450.5,"threshold":100.0,"subject":"0 C:"}}],"next_steps":["UpdateStorageDriver","CheckDiskCable"]}]},
] satisfies CaptureDto[];

export const captureNoteSamples = [
  {"id":4,"start_ms":1790000000000,"end_ms":1790000060000,"suspects":[{"cause":"Unclear","confidence":"Medium","evidence":[{"Event":{"ts_ms":1790000000000,"provider":"example-provider","id":129,"meaning":"StorageReset"}},{"Metric":{"ts_ms":1790000000000,"metric":"DiskLatencyMs","value":450.5,"threshold":100.0,"subject":"0 C:"}}],"next_steps":["UpdateStorageDriver","CheckDiskCable"]}]},
] satisfies CaptureNote[];

export const captureRunSamples = [
  {"started_ms":1790000000000,"ends_ms":1790000060000},
] satisfies CaptureRun[];

export const captureFinishedSamples = [
  {"ids":[4,5],"exit_code":0},
  {"ids":[4,5],"exit_code":null},
] satisfies CaptureFinished[];

export const concernSamples = [
  "Attention",
  "Critical",
] satisfies Concern[];

export const findingKindSamples = [
  "DiskSmart",
  "LowSpace",
  "PageFile",
  "MemoryOften",
  "DisplayResets",
  "HardwareErrors",
  "StorageErrors",
  "Throttling",
  "WindowsOnHdd",
  "TwoAntivirus",
  "SlowBoot",
  "DirtyRestarts",
  "ShutdownUnfinished",
  "FailuresRising",
  "NoCrashDump",
  "MemoryTestErrors",
  "DiskUnreadable",
  "VolumeErrors",
] satisfies FindingKind[];

export const subjectSamples = [
  "Pc",
  {"Disk":"0 C:"},
  {"Volume":"C:\\"},
] satisfies Subject[];

export const measureSamples = [
  {"Count":3},
  {"Percent":91.5},
  {"Days":12},
  {"Seconds":40.5},
  {"Bytes":1000000},
  {"Media":"Hdd"},
  {"Media":"Ssd"},
  {"Media":"Hdd"},
  {"Media":"Unknown"},
] satisfies Measure[];

export const referenceSamples = [
  {"AtMost":{"Percent":80.0}},
  {"AtLeast":{"Bytes":5000000}},
  {"Is":{"Media":"Ssd"}},
] satisfies Reference[];

export const smartAttributeSamples = [
  {"id":5,"current":100,"worst":99,"raw":12},
] satisfies SmartAttribute[];

export const nvmeHealthSamples = [
  {"critical_warning":0,"temperature_c":41,"available_spare_pct":100,"spare_threshold_pct":10,"percentage_used":3,"media_errors":0},
] satisfies NvmeHealth[];

export const pageFileSamples = [
  {"path":"C:\\pagefile.sys","size_bytes":4000000,"peak_bytes":3000000},
] satisfies PageFile[];

export const securityProductSamples = [
  {"name":"Example Antivirus","on":true},
  {"name":"Example Antivirus","on":false},
] satisfies SecurityProduct[];

export const delayKindSamples = [
  "App",
  "Driver",
  "Service",
  "Phase",
  "Device",
] satisfies DelayKind[];

export const bootTimeSamples = [
  {"ts_ms":1790000000000,"boot_ms":45000,"main_path_ms":30000,"post_boot_ms":15000},
] satisfies BootTime[];

export const bootDelaySamples = [
  {"ts_ms":1790000000000,"kind":"App","name":"example","total_ms":9000,"degradation_ms":4000,"security":false},
  {"ts_ms":1790000000000,"kind":"Driver","name":"example","total_ms":9000,"degradation_ms":4000,"security":false},
  {"ts_ms":1790000000000,"kind":"Service","name":"example","total_ms":9000,"degradation_ms":4000,"security":false},
  {"ts_ms":1790000000000,"kind":"Phase","name":"example","total_ms":9000,"degradation_ms":4000,"security":false},
  {"ts_ms":1790000000000,"kind":"Device","name":"example","total_ms":9000,"degradation_ms":4000,"security":false},
] satisfies BootDelay[];

export const bootReportSamples = [
  {"boots":[{"ts_ms":1790000000000,"boot_ms":45000,"main_path_ms":30000,"post_boot_ms":15000}],"delays":[{"ts_ms":1790000000000,"kind":"App","name":"example","total_ms":9000,"degradation_ms":4000,"security":false},{"ts_ms":1790000000000,"kind":"Driver","name":"example","total_ms":9000,"degradation_ms":4000,"security":false},{"ts_ms":1790000000000,"kind":"Service","name":"example","total_ms":9000,"degradation_ms":4000,"security":false},{"ts_ms":1790000000000,"kind":"Phase","name":"example","total_ms":9000,"degradation_ms":4000,"security":false},{"ts_ms":1790000000000,"kind":"Device","name":"example","total_ms":9000,"degradation_ms":4000,"security":false}],"skipped":2},
] satisfies BootReport[];

export const healthFactSamples = [
  {"Attribute":{"id":5,"current":100,"worst":99,"raw":12}},
  "PredictFailure",
  {"Nvme":{"critical_warning":0,"temperature_c":41,"available_spare_pct":100,"spare_threshold_pct":10,"percentage_used":3,"media_errors":0}},
  {"Event":{"ts_ms":1790000000000,"provider":"example-provider","id":153,"meaning":"DiskRetry"}},
  {"Space":{"free_bytes":5000000,"total_bytes":100000000}},
  {"PageFile":{"path":"C:\\pagefile.sys","size_bytes":4000000,"peak_bytes":3000000}},
  "NoPageFile",
  {"CommitPeak":{"pct":91.5}},
  {"MemoryHighDays":{"days":4}},
  {"Product":{"name":"Example Antivirus","on":true}},
  {"Boot":{"ts_ms":1790000000000,"boot_ms":45000,"main_path_ms":30000,"post_boot_ms":15000}},
  {"Delay":{"ts_ms":1790000000000,"kind":"App","name":"example","total_ms":9000,"degradation_ms":4000,"security":false}},
  {"Failures":{"last_week":5,"weekly_before":1.5}},
  {"SystemDisk":{"disk":"0 C:","media":"Hdd"}},
  {"MemoryDiagnostic":{"ts_ms":1790000000000,"bad_pages":2}},
  {"MemoryQuickTest":{"ts_ms":1790000000000,"errors":1,"tested_bytes":1000000}},
  {"SurfaceScan":{"ts_ms":1790000000000,"unreadable_blocks":3,"slow_blocks":8,"whole_disk":true}},
  {"VolumeCheck":{"ts_ms":1790000000000}},
  {"MemoryDiagnostic":{"ts_ms":1790000000000,"bad_pages":null}},
  {"Delay":{"ts_ms":1790000000000,"kind":"App","name":"example","total_ms":9000,"degradation_ms":4000,"security":false}},
  {"Delay":{"ts_ms":1790000000000,"kind":"Driver","name":"example","total_ms":9000,"degradation_ms":4000,"security":false}},
  {"Delay":{"ts_ms":1790000000000,"kind":"Service","name":"example","total_ms":9000,"degradation_ms":4000,"security":false}},
  {"Delay":{"ts_ms":1790000000000,"kind":"Phase","name":"example","total_ms":9000,"degradation_ms":4000,"security":false}},
  {"Delay":{"ts_ms":1790000000000,"kind":"Device","name":"example","total_ms":9000,"degradation_ms":4000,"security":false}},
  {"SystemDisk":{"disk":"0 C:","media":"Ssd"}},
  {"SystemDisk":{"disk":"0 C:","media":"Hdd"}},
  {"SystemDisk":{"disk":"0 C:","media":"Unknown"}},
] satisfies HealthFact[];

export const findingDtoSamples = [
  {"id":1,"opened_ms":1790000000000,"updated_ms":1790000100000,"closed_ms":1790000200000,"kind":"DiskSmart","subject":"Pc","concern":"Attention","value":{"Count":0},"reference":{"AtMost":{"Count":0}},"evidence":[],"next_steps":[],"first_value":{"Count":9}},
  {"id":2,"opened_ms":1790000000000,"updated_ms":1790000100000,"closed_ms":1790000200000,"kind":"LowSpace","subject":"Pc","concern":"Attention","value":{"Count":1},"reference":{"AtMost":{"Count":0}},"evidence":[],"next_steps":[],"first_value":{"Count":9}},
  {"id":3,"opened_ms":1790000000000,"updated_ms":1790000100000,"closed_ms":1790000200000,"kind":"PageFile","subject":"Pc","concern":"Attention","value":{"Count":2},"reference":{"AtMost":{"Count":0}},"evidence":[],"next_steps":[],"first_value":{"Count":9}},
  {"id":4,"opened_ms":1790000000000,"updated_ms":1790000100000,"closed_ms":1790000200000,"kind":"MemoryOften","subject":"Pc","concern":"Attention","value":{"Count":3},"reference":{"AtMost":{"Count":0}},"evidence":[],"next_steps":[],"first_value":{"Count":9}},
  {"id":5,"opened_ms":1790000000000,"updated_ms":1790000100000,"closed_ms":1790000200000,"kind":"DisplayResets","subject":"Pc","concern":"Attention","value":{"Count":4},"reference":{"AtMost":{"Count":0}},"evidence":[],"next_steps":[],"first_value":{"Count":9}},
  {"id":6,"opened_ms":1790000000000,"updated_ms":1790000100000,"closed_ms":1790000200000,"kind":"HardwareErrors","subject":"Pc","concern":"Attention","value":{"Count":5},"reference":{"AtMost":{"Count":0}},"evidence":[],"next_steps":[],"first_value":{"Count":9}},
  {"id":7,"opened_ms":1790000000000,"updated_ms":1790000100000,"closed_ms":1790000200000,"kind":"StorageErrors","subject":"Pc","concern":"Attention","value":{"Count":6},"reference":{"AtMost":{"Count":0}},"evidence":[],"next_steps":[],"first_value":{"Count":9}},
  {"id":8,"opened_ms":1790000000000,"updated_ms":1790000100000,"closed_ms":1790000200000,"kind":"Throttling","subject":"Pc","concern":"Attention","value":{"Count":7},"reference":{"AtMost":{"Count":0}},"evidence":[],"next_steps":[],"first_value":{"Count":9}},
  {"id":9,"opened_ms":1790000000000,"updated_ms":1790000100000,"closed_ms":1790000200000,"kind":"WindowsOnHdd","subject":"Pc","concern":"Attention","value":{"Count":8},"reference":{"AtMost":{"Count":0}},"evidence":[],"next_steps":[],"first_value":{"Count":9}},
  {"id":10,"opened_ms":1790000000000,"updated_ms":1790000100000,"closed_ms":1790000200000,"kind":"TwoAntivirus","subject":"Pc","concern":"Attention","value":{"Count":9},"reference":{"AtMost":{"Count":0}},"evidence":[],"next_steps":[],"first_value":{"Count":9}},
  {"id":11,"opened_ms":1790000000000,"updated_ms":1790000100000,"closed_ms":1790000200000,"kind":"SlowBoot","subject":"Pc","concern":"Attention","value":{"Count":10},"reference":{"AtMost":{"Count":0}},"evidence":[],"next_steps":[],"first_value":{"Count":9}},
  {"id":12,"opened_ms":1790000000000,"updated_ms":1790000100000,"closed_ms":1790000200000,"kind":"DirtyRestarts","subject":"Pc","concern":"Attention","value":{"Count":11},"reference":{"AtMost":{"Count":0}},"evidence":[],"next_steps":[],"first_value":{"Count":9}},
  {"id":13,"opened_ms":1790000000000,"updated_ms":1790000100000,"closed_ms":1790000200000,"kind":"ShutdownUnfinished","subject":"Pc","concern":"Attention","value":{"Count":12},"reference":{"AtMost":{"Count":0}},"evidence":[],"next_steps":[],"first_value":{"Count":9}},
  {"id":14,"opened_ms":1790000000000,"updated_ms":1790000100000,"closed_ms":1790000200000,"kind":"FailuresRising","subject":"Pc","concern":"Attention","value":{"Count":13},"reference":{"AtMost":{"Count":0}},"evidence":[],"next_steps":[],"first_value":{"Count":9}},
  {"id":15,"opened_ms":1790000000000,"updated_ms":1790000100000,"closed_ms":1790000200000,"kind":"NoCrashDump","subject":"Pc","concern":"Attention","value":{"Count":14},"reference":{"AtMost":{"Count":0}},"evidence":[],"next_steps":[],"first_value":{"Count":9}},
  {"id":16,"opened_ms":1790000000000,"updated_ms":1790000100000,"closed_ms":1790000200000,"kind":"MemoryTestErrors","subject":"Pc","concern":"Attention","value":{"Count":15},"reference":{"AtMost":{"Count":0}},"evidence":[],"next_steps":[],"first_value":{"Count":9}},
  {"id":17,"opened_ms":1790000000000,"updated_ms":1790000100000,"closed_ms":1790000200000,"kind":"DiskUnreadable","subject":"Pc","concern":"Attention","value":{"Count":16},"reference":{"AtMost":{"Count":0}},"evidence":[],"next_steps":[],"first_value":{"Count":9}},
  {"id":18,"opened_ms":1790000000000,"updated_ms":1790000100000,"closed_ms":1790000200000,"kind":"VolumeErrors","subject":"Pc","concern":"Attention","value":{"Count":17},"reference":{"AtMost":{"Count":0}},"evidence":[],"next_steps":[],"first_value":{"Count":9}},
  {"id":19,"opened_ms":1790000000000,"updated_ms":1790000100000,"closed_ms":1790000200000,"kind":"DiskSmart","subject":"Pc","concern":"Attention","value":{"Count":3},"reference":{"AtMost":{"Percent":80.0}},"evidence":[{"Attribute":{"id":5,"current":100,"worst":99,"raw":12}},"PredictFailure",{"Nvme":{"critical_warning":0,"temperature_c":41,"available_spare_pct":100,"spare_threshold_pct":10,"percentage_used":3,"media_errors":0}}],"next_steps":["UpdateStorageDriver","CheckDiskCable","RunDiskTest"],"first_value":{"Count":9}},
  {"id":20,"opened_ms":1790000000000,"updated_ms":1790000100000,"closed_ms":1790000200000,"kind":"LowSpace","subject":{"Disk":"0 C:"},"concern":"Critical","value":{"Percent":91.5},"reference":{"AtLeast":{"Bytes":5000000}},"evidence":[{"Attribute":{"id":5,"current":100,"worst":99,"raw":12}},"PredictFailure",{"Nvme":{"critical_warning":0,"temperature_c":41,"available_spare_pct":100,"spare_threshold_pct":10,"percentage_used":3,"media_errors":0}}],"next_steps":["UpdateStorageDriver","CheckDiskCable","RunDiskTest"],"first_value":{"Count":9}},
  {"id":21,"opened_ms":1790000000000,"updated_ms":1790000100000,"closed_ms":1790000200000,"kind":"PageFile","subject":{"Volume":"C:\\"},"concern":"Attention","value":{"Days":12},"reference":{"Is":{"Media":"Ssd"}},"evidence":[{"Attribute":{"id":5,"current":100,"worst":99,"raw":12}},"PredictFailure",{"Nvme":{"critical_warning":0,"temperature_c":41,"available_spare_pct":100,"spare_threshold_pct":10,"percentage_used":3,"media_errors":0}}],"next_steps":["UpdateStorageDriver","CheckDiskCable","RunDiskTest"],"first_value":{"Count":9}},
  {"id":1,"opened_ms":1790000000000,"updated_ms":1790000100000,"closed_ms":null,"kind":"DiskSmart","subject":"Pc","concern":"Attention","value":{"Count":0},"reference":{"AtMost":{"Count":0}},"evidence":[],"next_steps":[],"first_value":null},
  {"id":2,"opened_ms":1790000000000,"updated_ms":1790000100000,"closed_ms":null,"kind":"LowSpace","subject":"Pc","concern":"Attention","value":{"Count":1},"reference":{"AtMost":{"Count":0}},"evidence":[],"next_steps":[],"first_value":null},
  {"id":3,"opened_ms":1790000000000,"updated_ms":1790000100000,"closed_ms":null,"kind":"PageFile","subject":"Pc","concern":"Attention","value":{"Count":2},"reference":{"AtMost":{"Count":0}},"evidence":[],"next_steps":[],"first_value":null},
  {"id":4,"opened_ms":1790000000000,"updated_ms":1790000100000,"closed_ms":null,"kind":"MemoryOften","subject":"Pc","concern":"Attention","value":{"Count":3},"reference":{"AtMost":{"Count":0}},"evidence":[],"next_steps":[],"first_value":null},
  {"id":5,"opened_ms":1790000000000,"updated_ms":1790000100000,"closed_ms":null,"kind":"DisplayResets","subject":"Pc","concern":"Attention","value":{"Count":4},"reference":{"AtMost":{"Count":0}},"evidence":[],"next_steps":[],"first_value":null},
  {"id":6,"opened_ms":1790000000000,"updated_ms":1790000100000,"closed_ms":null,"kind":"HardwareErrors","subject":"Pc","concern":"Attention","value":{"Count":5},"reference":{"AtMost":{"Count":0}},"evidence":[],"next_steps":[],"first_value":null},
  {"id":7,"opened_ms":1790000000000,"updated_ms":1790000100000,"closed_ms":null,"kind":"StorageErrors","subject":"Pc","concern":"Attention","value":{"Count":6},"reference":{"AtMost":{"Count":0}},"evidence":[],"next_steps":[],"first_value":null},
  {"id":8,"opened_ms":1790000000000,"updated_ms":1790000100000,"closed_ms":null,"kind":"Throttling","subject":"Pc","concern":"Attention","value":{"Count":7},"reference":{"AtMost":{"Count":0}},"evidence":[],"next_steps":[],"first_value":null},
  {"id":9,"opened_ms":1790000000000,"updated_ms":1790000100000,"closed_ms":null,"kind":"WindowsOnHdd","subject":"Pc","concern":"Attention","value":{"Count":8},"reference":{"AtMost":{"Count":0}},"evidence":[],"next_steps":[],"first_value":null},
  {"id":10,"opened_ms":1790000000000,"updated_ms":1790000100000,"closed_ms":null,"kind":"TwoAntivirus","subject":"Pc","concern":"Attention","value":{"Count":9},"reference":{"AtMost":{"Count":0}},"evidence":[],"next_steps":[],"first_value":null},
  {"id":11,"opened_ms":1790000000000,"updated_ms":1790000100000,"closed_ms":null,"kind":"SlowBoot","subject":"Pc","concern":"Attention","value":{"Count":10},"reference":{"AtMost":{"Count":0}},"evidence":[],"next_steps":[],"first_value":null},
  {"id":12,"opened_ms":1790000000000,"updated_ms":1790000100000,"closed_ms":null,"kind":"DirtyRestarts","subject":"Pc","concern":"Attention","value":{"Count":11},"reference":{"AtMost":{"Count":0}},"evidence":[],"next_steps":[],"first_value":null},
  {"id":13,"opened_ms":1790000000000,"updated_ms":1790000100000,"closed_ms":null,"kind":"ShutdownUnfinished","subject":"Pc","concern":"Attention","value":{"Count":12},"reference":{"AtMost":{"Count":0}},"evidence":[],"next_steps":[],"first_value":null},
  {"id":14,"opened_ms":1790000000000,"updated_ms":1790000100000,"closed_ms":null,"kind":"FailuresRising","subject":"Pc","concern":"Attention","value":{"Count":13},"reference":{"AtMost":{"Count":0}},"evidence":[],"next_steps":[],"first_value":null},
  {"id":15,"opened_ms":1790000000000,"updated_ms":1790000100000,"closed_ms":null,"kind":"NoCrashDump","subject":"Pc","concern":"Attention","value":{"Count":14},"reference":{"AtMost":{"Count":0}},"evidence":[],"next_steps":[],"first_value":null},
  {"id":16,"opened_ms":1790000000000,"updated_ms":1790000100000,"closed_ms":null,"kind":"MemoryTestErrors","subject":"Pc","concern":"Attention","value":{"Count":15},"reference":{"AtMost":{"Count":0}},"evidence":[],"next_steps":[],"first_value":null},
  {"id":17,"opened_ms":1790000000000,"updated_ms":1790000100000,"closed_ms":null,"kind":"DiskUnreadable","subject":"Pc","concern":"Attention","value":{"Count":16},"reference":{"AtMost":{"Count":0}},"evidence":[],"next_steps":[],"first_value":null},
  {"id":18,"opened_ms":1790000000000,"updated_ms":1790000100000,"closed_ms":null,"kind":"VolumeErrors","subject":"Pc","concern":"Attention","value":{"Count":17},"reference":{"AtMost":{"Count":0}},"evidence":[],"next_steps":[],"first_value":null},
  {"id":19,"opened_ms":1790000000000,"updated_ms":1790000100000,"closed_ms":null,"kind":"DiskSmart","subject":"Pc","concern":"Attention","value":{"Count":3},"reference":{"AtMost":{"Percent":80.0}},"evidence":[{"Attribute":{"id":5,"current":100,"worst":99,"raw":12}},"PredictFailure",{"Nvme":{"critical_warning":0,"temperature_c":41,"available_spare_pct":100,"spare_threshold_pct":10,"percentage_used":3,"media_errors":0}}],"next_steps":["UpdateStorageDriver","CheckDiskCable","RunDiskTest"],"first_value":null},
  {"id":20,"opened_ms":1790000000000,"updated_ms":1790000100000,"closed_ms":null,"kind":"LowSpace","subject":{"Disk":"0 C:"},"concern":"Critical","value":{"Percent":91.5},"reference":{"AtLeast":{"Bytes":5000000}},"evidence":[{"Attribute":{"id":5,"current":100,"worst":99,"raw":12}},"PredictFailure",{"Nvme":{"critical_warning":0,"temperature_c":41,"available_spare_pct":100,"spare_threshold_pct":10,"percentage_used":3,"media_errors":0}}],"next_steps":["UpdateStorageDriver","CheckDiskCable","RunDiskTest"],"first_value":null},
  {"id":21,"opened_ms":1790000000000,"updated_ms":1790000100000,"closed_ms":null,"kind":"PageFile","subject":{"Volume":"C:\\"},"concern":"Attention","value":{"Days":12},"reference":{"Is":{"Media":"Ssd"}},"evidence":[{"Attribute":{"id":5,"current":100,"worst":99,"raw":12}},"PredictFailure",{"Nvme":{"critical_warning":0,"temperature_c":41,"available_spare_pct":100,"spare_threshold_pct":10,"percentage_used":3,"media_errors":0}}],"next_steps":["UpdateStorageDriver","CheckDiskCable","RunDiskTest"],"first_value":null},
] satisfies FindingDto[];

export const healthDtoSamples = [
  {"open":[{"id":1,"opened_ms":1790000000000,"updated_ms":1790000100000,"closed_ms":null,"kind":"DiskSmart","subject":"Pc","concern":"Attention","value":{"Count":0},"reference":{"AtMost":{"Count":0}},"evidence":[],"next_steps":[],"first_value":null},{"id":2,"opened_ms":1790000000000,"updated_ms":1790000100000,"closed_ms":null,"kind":"LowSpace","subject":"Pc","concern":"Attention","value":{"Count":1},"reference":{"AtMost":{"Count":0}},"evidence":[],"next_steps":[],"first_value":null}],"closed":[{"id":1,"opened_ms":1790000000000,"updated_ms":1790000100000,"closed_ms":1790000200000,"kind":"DiskSmart","subject":"Pc","concern":"Attention","value":{"Count":0},"reference":{"AtMost":{"Count":0}},"evidence":[],"next_steps":[],"first_value":{"Count":9}},{"id":2,"opened_ms":1790000000000,"updated_ms":1790000100000,"closed_ms":1790000200000,"kind":"LowSpace","subject":"Pc","concern":"Attention","value":{"Count":1},"reference":{"AtMost":{"Count":0}},"evidence":[],"next_steps":[],"first_value":{"Count":9}}],"last_scan_ms":1790000000000,"boot":{"boots":[{"ts_ms":1790000000000,"boot_ms":45000,"main_path_ms":30000,"post_boot_ms":15000}],"delays":[{"ts_ms":1790000000000,"kind":"App","name":"example","total_ms":9000,"degradation_ms":4000,"security":false},{"ts_ms":1790000000000,"kind":"Driver","name":"example","total_ms":9000,"degradation_ms":4000,"security":false},{"ts_ms":1790000000000,"kind":"Service","name":"example","total_ms":9000,"degradation_ms":4000,"security":false},{"ts_ms":1790000000000,"kind":"Phase","name":"example","total_ms":9000,"degradation_ms":4000,"security":false},{"ts_ms":1790000000000,"kind":"Device","name":"example","total_ms":9000,"degradation_ms":4000,"security":false}],"skipped":2}},
  {"open":[{"id":1,"opened_ms":1790000000000,"updated_ms":1790000100000,"closed_ms":null,"kind":"DiskSmart","subject":"Pc","concern":"Attention","value":{"Count":0},"reference":{"AtMost":{"Count":0}},"evidence":[],"next_steps":[],"first_value":null},{"id":2,"opened_ms":1790000000000,"updated_ms":1790000100000,"closed_ms":null,"kind":"LowSpace","subject":"Pc","concern":"Attention","value":{"Count":1},"reference":{"AtMost":{"Count":0}},"evidence":[],"next_steps":[],"first_value":null}],"closed":[{"id":1,"opened_ms":1790000000000,"updated_ms":1790000100000,"closed_ms":1790000200000,"kind":"DiskSmart","subject":"Pc","concern":"Attention","value":{"Count":0},"reference":{"AtMost":{"Count":0}},"evidence":[],"next_steps":[],"first_value":{"Count":9}},{"id":2,"opened_ms":1790000000000,"updated_ms":1790000100000,"closed_ms":1790000200000,"kind":"LowSpace","subject":"Pc","concern":"Attention","value":{"Count":1},"reference":{"AtMost":{"Count":0}},"evidence":[],"next_steps":[],"first_value":{"Count":9}}],"last_scan_ms":null,"boot":null},
] satisfies HealthDto[];

export const scanFinishedSamples = [
  {"scans":[1,2],"exit_code":0},
  {"scans":[1,2],"exit_code":null},
] satisfies ScanFinished[];

export const dumpKindSamples = [
  "KernelMini",
  "KernelMemory",
  "LiveKernel",
  "App",
] satisfies DumpKind[];

export const faultDtoSamples = [
  {"BugCheck":{"code":159,"params":["0x3","0x4","0x5","0x6"]}},
  {"Exception":{"code":3221225477,"address":"0x7ff612340000"}},
] satisfies FaultDto[];

export const dumpCulpritSamples = [
  {"Module":{"name":"example.sys","confidence":"High","intermediary":false}},
  "Hardware",
  {"App":{"name":"example.exe"}},
  {"Module":{"name":"example.sys","confidence":"Low","intermediary":true}},
  {"Module":{"name":"example.sys","confidence":"Medium","intermediary":true}},
  {"Module":{"name":"example.sys","confidence":"High","intermediary":true}},
] satisfies DumpCulprit[];

export const dumpDtoSamples = [
  {"id":7,"path":"C:\\Windows\\Minidump\\example.dmp","kind":"KernelMini","crash_ms":1790000000000,"file_ms":1790000001000,"size_bytes":2000000,"found_ms":1790000002000,"analyzed_ms":1790000003000,"deleted_ms":1790000004000,"fault":{"BugCheck":{"code":159,"params":["0x3","0x4","0x5","0x6"]}},"fault_module":"example.sys","stack_modules":["example.sys"],"process":"example.exe","module_count":120,"culprit":"Hardware"},
  {"id":7,"path":"C:\\Windows\\Minidump\\example.dmp","kind":"KernelMemory","crash_ms":1790000000000,"file_ms":1790000001000,"size_bytes":2000000,"found_ms":1790000002000,"analyzed_ms":1790000003000,"deleted_ms":1790000004000,"fault":{"BugCheck":{"code":159,"params":["0x3","0x4","0x5","0x6"]}},"fault_module":"example.sys","stack_modules":["example.sys"],"process":"example.exe","module_count":120,"culprit":"Hardware"},
  {"id":7,"path":"C:\\Windows\\Minidump\\example.dmp","kind":"LiveKernel","crash_ms":1790000000000,"file_ms":1790000001000,"size_bytes":2000000,"found_ms":1790000002000,"analyzed_ms":1790000003000,"deleted_ms":1790000004000,"fault":{"BugCheck":{"code":159,"params":["0x3","0x4","0x5","0x6"]}},"fault_module":"example.sys","stack_modules":["example.sys"],"process":"example.exe","module_count":120,"culprit":"Hardware"},
  {"id":7,"path":"C:\\Windows\\Minidump\\example.dmp","kind":"App","crash_ms":1790000000000,"file_ms":1790000001000,"size_bytes":2000000,"found_ms":1790000002000,"analyzed_ms":1790000003000,"deleted_ms":1790000004000,"fault":{"BugCheck":{"code":159,"params":["0x3","0x4","0x5","0x6"]}},"fault_module":"example.sys","stack_modules":["example.sys"],"process":"example.exe","module_count":120,"culprit":"Hardware"},
  {"id":7,"path":"C:\\Windows\\Minidump\\example.dmp","kind":"KernelMini","crash_ms":1790000000000,"file_ms":1790000001000,"size_bytes":2000000,"found_ms":1790000002000,"analyzed_ms":null,"deleted_ms":null,"fault":null,"fault_module":null,"stack_modules":["example.sys"],"process":null,"module_count":null,"culprit":null},
  {"id":7,"path":"C:\\Windows\\Minidump\\example.dmp","kind":"KernelMini","crash_ms":1790000000000,"file_ms":1790000001000,"size_bytes":2000000,"found_ms":1790000002000,"analyzed_ms":1790000003000,"deleted_ms":1790000004000,"fault":{"BugCheck":{"code":159,"params":["0x3","0x4","0x5","0x6"]}},"fault_module":"example.sys","stack_modules":["example.sys"],"process":"example.exe","module_count":120,"culprit":{"Module":{"name":"example.sys","confidence":"High","intermediary":false}}},
  {"id":7,"path":"C:\\Windows\\Minidump\\example.dmp","kind":"KernelMini","crash_ms":1790000000000,"file_ms":1790000001000,"size_bytes":2000000,"found_ms":1790000002000,"analyzed_ms":1790000003000,"deleted_ms":1790000004000,"fault":{"Exception":{"code":3221225477,"address":"0x7ff612340000"}},"fault_module":"example.sys","stack_modules":["example.sys"],"process":"example.exe","module_count":120,"culprit":"Hardware"},
  {"id":7,"path":"C:\\Windows\\Minidump\\example.dmp","kind":"App","crash_ms":1790000000000,"file_ms":1790000001000,"size_bytes":2000000,"found_ms":1790000002000,"analyzed_ms":1790000003000,"deleted_ms":1790000004000,"fault":{"BugCheck":{"code":159,"params":["0x3","0x4","0x5","0x6"]}},"fault_module":"example.sys","stack_modules":["example.sys"],"process":"example.exe","module_count":120,"culprit":{"App":{"name":"example.exe"}}},
  {"id":7,"path":"C:\\Windows\\Minidump\\example.dmp","kind":"App","crash_ms":1790000000000,"file_ms":1790000001000,"size_bytes":2000000,"found_ms":1790000002000,"analyzed_ms":1790000003000,"deleted_ms":1790000004000,"fault":{"BugCheck":{"code":159,"params":["0x3","0x4","0x5","0x6"]}},"fault_module":"example.sys","stack_modules":["example.sys"],"process":"example.exe","module_count":120,"culprit":{"Module":{"name":"example.sys","confidence":"Low","intermediary":true}}},
  {"id":7,"path":"C:\\Windows\\Minidump\\example.dmp","kind":"App","crash_ms":1790000000000,"file_ms":1790000001000,"size_bytes":2000000,"found_ms":1790000002000,"analyzed_ms":1790000003000,"deleted_ms":1790000004000,"fault":{"BugCheck":{"code":159,"params":["0x3","0x4","0x5","0x6"]}},"fault_module":"example.sys","stack_modules":["example.sys"],"process":"example.exe","module_count":120,"culprit":{"Module":{"name":"example.sys","confidence":"Medium","intermediary":true}}},
  {"id":7,"path":"C:\\Windows\\Minidump\\example.dmp","kind":"App","crash_ms":1790000000000,"file_ms":1790000001000,"size_bytes":2000000,"found_ms":1790000002000,"analyzed_ms":1790000003000,"deleted_ms":1790000004000,"fault":{"BugCheck":{"code":159,"params":["0x3","0x4","0x5","0x6"]}},"fault_module":"example.sys","stack_modules":["example.sys"],"process":"example.exe","module_count":120,"culprit":{"Module":{"name":"example.sys","confidence":"High","intermediary":true}}},
] satisfies DumpDto[];

export const bugCheckDtoSamples = [
  {"ts_ms":1790000000000,"code":80,"params":["0x1","0x2","0x3","0x4"],"dump_path":"C:\\Windows\\MEMORY.DMP"},
] satisfies BugCheckDto[];

export const crashControlDtoSamples = [
  {"dump_type":7,"dump_file":"C:\\Windows\\MEMORY.DMP","minidump_dir":"C:\\Windows\\Minidump","overwrite":false},
] satisfies CrashControlDto[];

export const dumpsDtoSamples = [
  {"dumps":[{"id":7,"path":"C:\\Windows\\Minidump\\example.dmp","kind":"KernelMini","crash_ms":1790000000000,"file_ms":1790000001000,"size_bytes":2000000,"found_ms":1790000002000,"analyzed_ms":1790000003000,"deleted_ms":1790000004000,"fault":{"BugCheck":{"code":159,"params":["0x3","0x4","0x5","0x6"]}},"fault_module":"example.sys","stack_modules":["example.sys"],"process":"example.exe","module_count":120,"culprit":"Hardware"},{"id":7,"path":"C:\\Windows\\Minidump\\example.dmp","kind":"KernelMemory","crash_ms":1790000000000,"file_ms":1790000001000,"size_bytes":2000000,"found_ms":1790000002000,"analyzed_ms":1790000003000,"deleted_ms":1790000004000,"fault":{"BugCheck":{"code":159,"params":["0x3","0x4","0x5","0x6"]}},"fault_module":"example.sys","stack_modules":["example.sys"],"process":"example.exe","module_count":120,"culprit":"Hardware"}],"space_bytes":4000000,"reports":[{"ts_ms":1790000000000,"code":80,"params":["0x1","0x2","0x3","0x4"],"dump_path":""}],"crash":{"dump_type":7,"dump_file":"C:\\Windows\\MEMORY.DMP","minidump_dir":"C:\\Windows\\Minidump","overwrite":true}},
  {"dumps":[{"id":7,"path":"C:\\Windows\\Minidump\\example.dmp","kind":"KernelMini","crash_ms":1790000000000,"file_ms":1790000001000,"size_bytes":2000000,"found_ms":1790000002000,"analyzed_ms":1790000003000,"deleted_ms":1790000004000,"fault":{"BugCheck":{"code":159,"params":["0x3","0x4","0x5","0x6"]}},"fault_module":"example.sys","stack_modules":["example.sys"],"process":"example.exe","module_count":120,"culprit":"Hardware"},{"id":7,"path":"C:\\Windows\\Minidump\\example.dmp","kind":"KernelMemory","crash_ms":1790000000000,"file_ms":1790000001000,"size_bytes":2000000,"found_ms":1790000002000,"analyzed_ms":1790000003000,"deleted_ms":1790000004000,"fault":{"BugCheck":{"code":159,"params":["0x3","0x4","0x5","0x6"]}},"fault_module":"example.sys","stack_modules":["example.sys"],"process":"example.exe","module_count":120,"culprit":"Hardware"}],"space_bytes":4000000,"reports":[{"ts_ms":1790000000000,"code":80,"params":["0x1","0x2","0x3","0x4"],"dump_path":""}],"crash":null},
] satisfies DumpsDto[];

export const dumpScanFinishedSamples = [
  {"dumps":[7],"exit_code":0},
  {"dumps":[7],"exit_code":null},
] satisfies DumpScanFinished[];

export const deleteFinishedSamples = [
  {"deleted":2,"refused":1,"failed":0,"exit_code":0},
  {"deleted":2,"refused":1,"failed":0,"exit_code":null},
] satisfies DeleteFinished[];

export const tweakIdSamples = [
  "FastStartup",
  "Hibernation",
  "PowerPlan",
  "PcieAspm",
  "UsbSuspend",
  "SysMain",
  "WindowsSearch",
  "GameDvr",
  "GpuScheduling",
  "Mpo",
  "StartupApp",
  "AppDumps",
  "CrashDump",
] satisfies TweakId[];

export const hiveSamples = [
  "LocalMachine",
  "CurrentUser",
] satisfies Hive[];

export const startupListSamples = [
  "Run",
  "Run32",
  "StartupFolder",
] satisfies StartupList[];

export const tweakSettingSamples = [
  {"Dword":{"hive":"LocalMachine","key":"SOFTWARE\\Example","name":"Enabled"}},
  {"Key":{"hive":"CurrentUser","key":"SOFTWARE\\Example"}},
  {"Service":{"name":"Example"}},
  {"PowerAc":{"subgroup":"2a737441-1930-4402-8d77-b2bebba30001","setting":"2a737441-1930-4402-8d77-b2bebba30002","scheme":"2a737441-1930-4402-8d77-b2bebba30003"}},
  "ActiveScheme",
  "Hibernation",
  {"StartupApproval":{"hive":"CurrentUser","list":"Run","name":"Example"}},
  {"PowerAc":{"subgroup":"2a737441-1930-4402-8d77-b2bebba30001","setting":"2a737441-1930-4402-8d77-b2bebba30002","scheme":null}},
  {"StartupApproval":{"hive":"LocalMachine","list":"Run","name":"Example"}},
  {"StartupApproval":{"hive":"LocalMachine","list":"Run32","name":"Example"}},
  {"StartupApproval":{"hive":"LocalMachine","list":"StartupFolder","name":"Example"}},
] satisfies TweakSetting[];

export const tweakValueSamples = [
  "Absent",
  {"Dword":1},
  "Present",
  {"Service":{"start":2,"delayed":true,"running":false}},
  {"Index":1},
  {"Scheme":"2a737441-1930-4402-8d77-b2bebba30004"},
  {"On":true},
  {"Bytes":[1,2,3]},
] satisfies TweakValue[];

export const settingChangeSamples = [
  {"setting":{"Dword":{"hive":"LocalMachine","key":"SOFTWARE\\Example","name":"Enabled"}},"from":"Absent","to":"Absent"},
  {"setting":{"Key":{"hive":"CurrentUser","key":"SOFTWARE\\Example"}},"from":{"Dword":1},"to":"Absent"},
  {"setting":{"Service":{"name":"Example"}},"from":"Present","to":"Absent"},
  {"setting":{"PowerAc":{"subgroup":"2a737441-1930-4402-8d77-b2bebba30001","setting":"2a737441-1930-4402-8d77-b2bebba30002","scheme":"2a737441-1930-4402-8d77-b2bebba30003"}},"from":{"Service":{"start":2,"delayed":true,"running":false}},"to":"Absent"},
  {"setting":"ActiveScheme","from":{"Index":1},"to":"Absent"},
  {"setting":"Hibernation","from":{"Scheme":"2a737441-1930-4402-8d77-b2bebba30004"},"to":"Absent"},
  {"setting":{"StartupApproval":{"hive":"CurrentUser","list":"Run","name":"Example"}},"from":{"On":true},"to":"Absent"},
  {"setting":{"PowerAc":{"subgroup":"2a737441-1930-4402-8d77-b2bebba30001","setting":"2a737441-1930-4402-8d77-b2bebba30002","scheme":null}},"from":{"Bytes":[1,2,3]},"to":"Absent"},
  {"setting":{"StartupApproval":{"hive":"LocalMachine","list":"Run","name":"Example"}},"from":"Absent","to":"Absent"},
  {"setting":{"StartupApproval":{"hive":"LocalMachine","list":"Run32","name":"Example"}},"from":{"Dword":1},"to":"Absent"},
  {"setting":{"StartupApproval":{"hive":"LocalMachine","list":"StartupFolder","name":"Example"}},"from":"Present","to":"Absent"},
] satisfies SettingChange[];

export const tweakSignalSamples = [
  {"Finding":"SlowBoot"},
  {"UnfinishedShutdowns":{"count":3}},
  {"SleepFalls":{"count":2}},
  {"Stutters":{"count":6}},
  {"DpcIsrStalls":{"count":4}},
  {"GpuDriverStalls":{"count":1}},
  {"DiskBusy":{"count":5,"culprit":"example.exe"}},
  {"CpuSaturated":{"count":5,"culprit":"example.exe"}},
  {"CaptureDriver":{"start_ms":1790000000000,"module":"example.sys"}},
  {"CaptureFrames":{"start_ms":1790000000000,"app":"example.exe"}},
  {"AppCrashes":{"count":3}},
  {"HiberFile":{"bytes":8000000000}},
  {"Monitors":{"count":2}},
  "DumpsOff",
  {"SlowStartup":{"name":"example.exe","total_ms":9000}},
  {"DiskBusy":{"count":5,"culprit":null}},
  {"CpuSaturated":{"count":5,"culprit":null}},
  {"Finding":"DiskSmart"},
  {"Finding":"LowSpace"},
  {"Finding":"PageFile"},
  {"Finding":"MemoryOften"},
  {"Finding":"DisplayResets"},
  {"Finding":"HardwareErrors"},
  {"Finding":"StorageErrors"},
  {"Finding":"Throttling"},
  {"Finding":"WindowsOnHdd"},
  {"Finding":"TwoAntivirus"},
  {"Finding":"SlowBoot"},
  {"Finding":"DirtyRestarts"},
  {"Finding":"ShutdownUnfinished"},
  {"Finding":"FailuresRising"},
  {"Finding":"NoCrashDump"},
  {"Finding":"MemoryTestErrors"},
  {"Finding":"DiskUnreadable"},
  {"Finding":"VolumeErrors"},
] satisfies TweakSignal[];

export const tweakStatusSamples = [
  "Ready",
  "AlreadySet",
  "Unavailable",
  "Protected",
] satisfies TweakStatus[];

export const applierSamples = [
  "User",
  "Elevated",
] satisfies Applier[];

export const takesSamples = [
  "Now",
  "NextShutdown",
  "NextSignIn",
  "Restart",
] satisfies Takes[];

export const adviceSamples = [
  {"tweak":"FastStartup","subject":"Example","status":"Ready","signals":[{"Finding":"SlowBoot"},{"UnfinishedShutdowns":{"count":3}},{"SleepFalls":{"count":2}}],"changes":[{"setting":{"Dword":{"hive":"LocalMachine","key":"SOFTWARE\\Example","name":"Enabled"}},"from":"Absent","to":"Absent"},{"setting":{"Key":{"hive":"CurrentUser","key":"SOFTWARE\\Example"}},"from":{"Dword":1},"to":"Absent"}],"current":[{"setting":{"Dword":{"hive":"LocalMachine","key":"SOFTWARE\\Example","name":"Enabled"}},"value":"Absent"},{"setting":{"Key":{"hive":"CurrentUser","key":"SOFTWARE\\Example"}},"value":{"Dword":1}}],"applier":"User","takes":"Now"},
  {"tweak":"Hibernation","subject":"Example","status":"AlreadySet","signals":[{"Finding":"SlowBoot"},{"UnfinishedShutdowns":{"count":3}},{"SleepFalls":{"count":2}}],"changes":[{"setting":{"Dword":{"hive":"LocalMachine","key":"SOFTWARE\\Example","name":"Enabled"}},"from":"Absent","to":"Absent"},{"setting":{"Key":{"hive":"CurrentUser","key":"SOFTWARE\\Example"}},"from":{"Dword":1},"to":"Absent"}],"current":[{"setting":{"Dword":{"hive":"LocalMachine","key":"SOFTWARE\\Example","name":"Enabled"}},"value":"Absent"},{"setting":{"Key":{"hive":"CurrentUser","key":"SOFTWARE\\Example"}},"value":{"Dword":1}}],"applier":"Elevated","takes":"NextShutdown"},
  {"tweak":"PowerPlan","subject":"Example","status":"Unavailable","signals":[{"Finding":"SlowBoot"},{"UnfinishedShutdowns":{"count":3}},{"SleepFalls":{"count":2}}],"changes":[{"setting":{"Dword":{"hive":"LocalMachine","key":"SOFTWARE\\Example","name":"Enabled"}},"from":"Absent","to":"Absent"},{"setting":{"Key":{"hive":"CurrentUser","key":"SOFTWARE\\Example"}},"from":{"Dword":1},"to":"Absent"}],"current":[{"setting":{"Dword":{"hive":"LocalMachine","key":"SOFTWARE\\Example","name":"Enabled"}},"value":"Absent"},{"setting":{"Key":{"hive":"CurrentUser","key":"SOFTWARE\\Example"}},"value":{"Dword":1}}],"applier":"User","takes":"NextSignIn"},
  {"tweak":"PcieAspm","subject":"Example","status":"Protected","signals":[{"Finding":"SlowBoot"},{"UnfinishedShutdowns":{"count":3}},{"SleepFalls":{"count":2}}],"changes":[{"setting":{"Dword":{"hive":"LocalMachine","key":"SOFTWARE\\Example","name":"Enabled"}},"from":"Absent","to":"Absent"},{"setting":{"Key":{"hive":"CurrentUser","key":"SOFTWARE\\Example"}},"from":{"Dword":1},"to":"Absent"}],"current":[{"setting":{"Dword":{"hive":"LocalMachine","key":"SOFTWARE\\Example","name":"Enabled"}},"value":"Absent"},{"setting":{"Key":{"hive":"CurrentUser","key":"SOFTWARE\\Example"}},"value":{"Dword":1}}],"applier":"Elevated","takes":"Restart"},
  {"tweak":"UsbSuspend","subject":"Example","status":"Ready","signals":[{"Finding":"SlowBoot"},{"UnfinishedShutdowns":{"count":3}},{"SleepFalls":{"count":2}}],"changes":[{"setting":{"Dword":{"hive":"LocalMachine","key":"SOFTWARE\\Example","name":"Enabled"}},"from":"Absent","to":"Absent"},{"setting":{"Key":{"hive":"CurrentUser","key":"SOFTWARE\\Example"}},"from":{"Dword":1},"to":"Absent"}],"current":[{"setting":{"Dword":{"hive":"LocalMachine","key":"SOFTWARE\\Example","name":"Enabled"}},"value":"Absent"},{"setting":{"Key":{"hive":"CurrentUser","key":"SOFTWARE\\Example"}},"value":{"Dword":1}}],"applier":"User","takes":"Now"},
  {"tweak":"SysMain","subject":"Example","status":"AlreadySet","signals":[{"Finding":"SlowBoot"},{"UnfinishedShutdowns":{"count":3}},{"SleepFalls":{"count":2}}],"changes":[{"setting":{"Dword":{"hive":"LocalMachine","key":"SOFTWARE\\Example","name":"Enabled"}},"from":"Absent","to":"Absent"},{"setting":{"Key":{"hive":"CurrentUser","key":"SOFTWARE\\Example"}},"from":{"Dword":1},"to":"Absent"}],"current":[{"setting":{"Dword":{"hive":"LocalMachine","key":"SOFTWARE\\Example","name":"Enabled"}},"value":"Absent"},{"setting":{"Key":{"hive":"CurrentUser","key":"SOFTWARE\\Example"}},"value":{"Dword":1}}],"applier":"Elevated","takes":"NextShutdown"},
  {"tweak":"WindowsSearch","subject":"Example","status":"Unavailable","signals":[{"Finding":"SlowBoot"},{"UnfinishedShutdowns":{"count":3}},{"SleepFalls":{"count":2}}],"changes":[{"setting":{"Dword":{"hive":"LocalMachine","key":"SOFTWARE\\Example","name":"Enabled"}},"from":"Absent","to":"Absent"},{"setting":{"Key":{"hive":"CurrentUser","key":"SOFTWARE\\Example"}},"from":{"Dword":1},"to":"Absent"}],"current":[{"setting":{"Dword":{"hive":"LocalMachine","key":"SOFTWARE\\Example","name":"Enabled"}},"value":"Absent"},{"setting":{"Key":{"hive":"CurrentUser","key":"SOFTWARE\\Example"}},"value":{"Dword":1}}],"applier":"User","takes":"NextSignIn"},
  {"tweak":"GameDvr","subject":"Example","status":"Protected","signals":[{"Finding":"SlowBoot"},{"UnfinishedShutdowns":{"count":3}},{"SleepFalls":{"count":2}}],"changes":[{"setting":{"Dword":{"hive":"LocalMachine","key":"SOFTWARE\\Example","name":"Enabled"}},"from":"Absent","to":"Absent"},{"setting":{"Key":{"hive":"CurrentUser","key":"SOFTWARE\\Example"}},"from":{"Dword":1},"to":"Absent"}],"current":[{"setting":{"Dword":{"hive":"LocalMachine","key":"SOFTWARE\\Example","name":"Enabled"}},"value":"Absent"},{"setting":{"Key":{"hive":"CurrentUser","key":"SOFTWARE\\Example"}},"value":{"Dword":1}}],"applier":"Elevated","takes":"Restart"},
  {"tweak":"GpuScheduling","subject":"Example","status":"Ready","signals":[{"Finding":"SlowBoot"},{"UnfinishedShutdowns":{"count":3}},{"SleepFalls":{"count":2}}],"changes":[{"setting":{"Dword":{"hive":"LocalMachine","key":"SOFTWARE\\Example","name":"Enabled"}},"from":"Absent","to":"Absent"},{"setting":{"Key":{"hive":"CurrentUser","key":"SOFTWARE\\Example"}},"from":{"Dword":1},"to":"Absent"}],"current":[{"setting":{"Dword":{"hive":"LocalMachine","key":"SOFTWARE\\Example","name":"Enabled"}},"value":"Absent"},{"setting":{"Key":{"hive":"CurrentUser","key":"SOFTWARE\\Example"}},"value":{"Dword":1}}],"applier":"User","takes":"Now"},
  {"tweak":"Mpo","subject":"Example","status":"AlreadySet","signals":[{"Finding":"SlowBoot"},{"UnfinishedShutdowns":{"count":3}},{"SleepFalls":{"count":2}}],"changes":[{"setting":{"Dword":{"hive":"LocalMachine","key":"SOFTWARE\\Example","name":"Enabled"}},"from":"Absent","to":"Absent"},{"setting":{"Key":{"hive":"CurrentUser","key":"SOFTWARE\\Example"}},"from":{"Dword":1},"to":"Absent"}],"current":[{"setting":{"Dword":{"hive":"LocalMachine","key":"SOFTWARE\\Example","name":"Enabled"}},"value":"Absent"},{"setting":{"Key":{"hive":"CurrentUser","key":"SOFTWARE\\Example"}},"value":{"Dword":1}}],"applier":"Elevated","takes":"NextShutdown"},
  {"tweak":"StartupApp","subject":"Example","status":"Unavailable","signals":[{"Finding":"SlowBoot"},{"UnfinishedShutdowns":{"count":3}},{"SleepFalls":{"count":2}}],"changes":[{"setting":{"Dword":{"hive":"LocalMachine","key":"SOFTWARE\\Example","name":"Enabled"}},"from":"Absent","to":"Absent"},{"setting":{"Key":{"hive":"CurrentUser","key":"SOFTWARE\\Example"}},"from":{"Dword":1},"to":"Absent"}],"current":[{"setting":{"Dword":{"hive":"LocalMachine","key":"SOFTWARE\\Example","name":"Enabled"}},"value":"Absent"},{"setting":{"Key":{"hive":"CurrentUser","key":"SOFTWARE\\Example"}},"value":{"Dword":1}}],"applier":"User","takes":"NextSignIn"},
  {"tweak":"AppDumps","subject":"Example","status":"Protected","signals":[{"Finding":"SlowBoot"},{"UnfinishedShutdowns":{"count":3}},{"SleepFalls":{"count":2}}],"changes":[{"setting":{"Dword":{"hive":"LocalMachine","key":"SOFTWARE\\Example","name":"Enabled"}},"from":"Absent","to":"Absent"},{"setting":{"Key":{"hive":"CurrentUser","key":"SOFTWARE\\Example"}},"from":{"Dword":1},"to":"Absent"}],"current":[{"setting":{"Dword":{"hive":"LocalMachine","key":"SOFTWARE\\Example","name":"Enabled"}},"value":"Absent"},{"setting":{"Key":{"hive":"CurrentUser","key":"SOFTWARE\\Example"}},"value":{"Dword":1}}],"applier":"Elevated","takes":"Restart"},
  {"tweak":"CrashDump","subject":"Example","status":"Ready","signals":[{"Finding":"SlowBoot"},{"UnfinishedShutdowns":{"count":3}},{"SleepFalls":{"count":2}}],"changes":[{"setting":{"Dword":{"hive":"LocalMachine","key":"SOFTWARE\\Example","name":"Enabled"}},"from":"Absent","to":"Absent"},{"setting":{"Key":{"hive":"CurrentUser","key":"SOFTWARE\\Example"}},"from":{"Dword":1},"to":"Absent"}],"current":[{"setting":{"Dword":{"hive":"LocalMachine","key":"SOFTWARE\\Example","name":"Enabled"}},"value":"Absent"},{"setting":{"Key":{"hive":"CurrentUser","key":"SOFTWARE\\Example"}},"value":{"Dword":1}}],"applier":"User","takes":"Now"},
  {"tweak":"FastStartup","subject":null,"status":"Ready","signals":[],"changes":[{"setting":{"Dword":{"hive":"LocalMachine","key":"SOFTWARE\\Example","name":"Enabled"}},"from":"Absent","to":"Absent"},{"setting":{"Key":{"hive":"CurrentUser","key":"SOFTWARE\\Example"}},"from":{"Dword":1},"to":"Absent"}],"current":[{"setting":{"Dword":{"hive":"LocalMachine","key":"SOFTWARE\\Example","name":"Enabled"}},"value":"Absent"},{"setting":{"Key":{"hive":"CurrentUser","key":"SOFTWARE\\Example"}},"value":{"Dword":1}}],"applier":"User","takes":"Now"},
  {"tweak":"Hibernation","subject":null,"status":"AlreadySet","signals":[],"changes":[{"setting":{"Dword":{"hive":"LocalMachine","key":"SOFTWARE\\Example","name":"Enabled"}},"from":"Absent","to":"Absent"},{"setting":{"Key":{"hive":"CurrentUser","key":"SOFTWARE\\Example"}},"from":{"Dword":1},"to":"Absent"}],"current":[{"setting":{"Dword":{"hive":"LocalMachine","key":"SOFTWARE\\Example","name":"Enabled"}},"value":"Absent"},{"setting":{"Key":{"hive":"CurrentUser","key":"SOFTWARE\\Example"}},"value":{"Dword":1}}],"applier":"Elevated","takes":"NextShutdown"},
  {"tweak":"PowerPlan","subject":null,"status":"Unavailable","signals":[],"changes":[{"setting":{"Dword":{"hive":"LocalMachine","key":"SOFTWARE\\Example","name":"Enabled"}},"from":"Absent","to":"Absent"},{"setting":{"Key":{"hive":"CurrentUser","key":"SOFTWARE\\Example"}},"from":{"Dword":1},"to":"Absent"}],"current":[{"setting":{"Dword":{"hive":"LocalMachine","key":"SOFTWARE\\Example","name":"Enabled"}},"value":"Absent"},{"setting":{"Key":{"hive":"CurrentUser","key":"SOFTWARE\\Example"}},"value":{"Dword":1}}],"applier":"User","takes":"NextSignIn"},
  {"tweak":"PcieAspm","subject":null,"status":"Protected","signals":[],"changes":[{"setting":{"Dword":{"hive":"LocalMachine","key":"SOFTWARE\\Example","name":"Enabled"}},"from":"Absent","to":"Absent"},{"setting":{"Key":{"hive":"CurrentUser","key":"SOFTWARE\\Example"}},"from":{"Dword":1},"to":"Absent"}],"current":[{"setting":{"Dword":{"hive":"LocalMachine","key":"SOFTWARE\\Example","name":"Enabled"}},"value":"Absent"},{"setting":{"Key":{"hive":"CurrentUser","key":"SOFTWARE\\Example"}},"value":{"Dword":1}}],"applier":"Elevated","takes":"Restart"},
  {"tweak":"UsbSuspend","subject":null,"status":"Ready","signals":[],"changes":[{"setting":{"Dword":{"hive":"LocalMachine","key":"SOFTWARE\\Example","name":"Enabled"}},"from":"Absent","to":"Absent"},{"setting":{"Key":{"hive":"CurrentUser","key":"SOFTWARE\\Example"}},"from":{"Dword":1},"to":"Absent"}],"current":[{"setting":{"Dword":{"hive":"LocalMachine","key":"SOFTWARE\\Example","name":"Enabled"}},"value":"Absent"},{"setting":{"Key":{"hive":"CurrentUser","key":"SOFTWARE\\Example"}},"value":{"Dword":1}}],"applier":"User","takes":"Now"},
  {"tweak":"SysMain","subject":null,"status":"AlreadySet","signals":[],"changes":[{"setting":{"Dword":{"hive":"LocalMachine","key":"SOFTWARE\\Example","name":"Enabled"}},"from":"Absent","to":"Absent"},{"setting":{"Key":{"hive":"CurrentUser","key":"SOFTWARE\\Example"}},"from":{"Dword":1},"to":"Absent"}],"current":[{"setting":{"Dword":{"hive":"LocalMachine","key":"SOFTWARE\\Example","name":"Enabled"}},"value":"Absent"},{"setting":{"Key":{"hive":"CurrentUser","key":"SOFTWARE\\Example"}},"value":{"Dword":1}}],"applier":"Elevated","takes":"NextShutdown"},
  {"tweak":"WindowsSearch","subject":null,"status":"Unavailable","signals":[],"changes":[{"setting":{"Dword":{"hive":"LocalMachine","key":"SOFTWARE\\Example","name":"Enabled"}},"from":"Absent","to":"Absent"},{"setting":{"Key":{"hive":"CurrentUser","key":"SOFTWARE\\Example"}},"from":{"Dword":1},"to":"Absent"}],"current":[{"setting":{"Dword":{"hive":"LocalMachine","key":"SOFTWARE\\Example","name":"Enabled"}},"value":"Absent"},{"setting":{"Key":{"hive":"CurrentUser","key":"SOFTWARE\\Example"}},"value":{"Dword":1}}],"applier":"User","takes":"NextSignIn"},
  {"tweak":"GameDvr","subject":null,"status":"Protected","signals":[],"changes":[{"setting":{"Dword":{"hive":"LocalMachine","key":"SOFTWARE\\Example","name":"Enabled"}},"from":"Absent","to":"Absent"},{"setting":{"Key":{"hive":"CurrentUser","key":"SOFTWARE\\Example"}},"from":{"Dword":1},"to":"Absent"}],"current":[{"setting":{"Dword":{"hive":"LocalMachine","key":"SOFTWARE\\Example","name":"Enabled"}},"value":"Absent"},{"setting":{"Key":{"hive":"CurrentUser","key":"SOFTWARE\\Example"}},"value":{"Dword":1}}],"applier":"Elevated","takes":"Restart"},
  {"tweak":"GpuScheduling","subject":null,"status":"Ready","signals":[],"changes":[{"setting":{"Dword":{"hive":"LocalMachine","key":"SOFTWARE\\Example","name":"Enabled"}},"from":"Absent","to":"Absent"},{"setting":{"Key":{"hive":"CurrentUser","key":"SOFTWARE\\Example"}},"from":{"Dword":1},"to":"Absent"}],"current":[{"setting":{"Dword":{"hive":"LocalMachine","key":"SOFTWARE\\Example","name":"Enabled"}},"value":"Absent"},{"setting":{"Key":{"hive":"CurrentUser","key":"SOFTWARE\\Example"}},"value":{"Dword":1}}],"applier":"User","takes":"Now"},
  {"tweak":"Mpo","subject":null,"status":"AlreadySet","signals":[],"changes":[{"setting":{"Dword":{"hive":"LocalMachine","key":"SOFTWARE\\Example","name":"Enabled"}},"from":"Absent","to":"Absent"},{"setting":{"Key":{"hive":"CurrentUser","key":"SOFTWARE\\Example"}},"from":{"Dword":1},"to":"Absent"}],"current":[{"setting":{"Dword":{"hive":"LocalMachine","key":"SOFTWARE\\Example","name":"Enabled"}},"value":"Absent"},{"setting":{"Key":{"hive":"CurrentUser","key":"SOFTWARE\\Example"}},"value":{"Dword":1}}],"applier":"Elevated","takes":"NextShutdown"},
  {"tweak":"StartupApp","subject":null,"status":"Unavailable","signals":[],"changes":[{"setting":{"Dword":{"hive":"LocalMachine","key":"SOFTWARE\\Example","name":"Enabled"}},"from":"Absent","to":"Absent"},{"setting":{"Key":{"hive":"CurrentUser","key":"SOFTWARE\\Example"}},"from":{"Dword":1},"to":"Absent"}],"current":[{"setting":{"Dword":{"hive":"LocalMachine","key":"SOFTWARE\\Example","name":"Enabled"}},"value":"Absent"},{"setting":{"Key":{"hive":"CurrentUser","key":"SOFTWARE\\Example"}},"value":{"Dword":1}}],"applier":"User","takes":"NextSignIn"},
  {"tweak":"AppDumps","subject":null,"status":"Protected","signals":[],"changes":[{"setting":{"Dword":{"hive":"LocalMachine","key":"SOFTWARE\\Example","name":"Enabled"}},"from":"Absent","to":"Absent"},{"setting":{"Key":{"hive":"CurrentUser","key":"SOFTWARE\\Example"}},"from":{"Dword":1},"to":"Absent"}],"current":[{"setting":{"Dword":{"hive":"LocalMachine","key":"SOFTWARE\\Example","name":"Enabled"}},"value":"Absent"},{"setting":{"Key":{"hive":"CurrentUser","key":"SOFTWARE\\Example"}},"value":{"Dword":1}}],"applier":"Elevated","takes":"Restart"},
  {"tweak":"CrashDump","subject":null,"status":"Ready","signals":[],"changes":[{"setting":{"Dword":{"hive":"LocalMachine","key":"SOFTWARE\\Example","name":"Enabled"}},"from":"Absent","to":"Absent"},{"setting":{"Key":{"hive":"CurrentUser","key":"SOFTWARE\\Example"}},"from":{"Dword":1},"to":"Absent"}],"current":[{"setting":{"Dword":{"hive":"LocalMachine","key":"SOFTWARE\\Example","name":"Enabled"}},"value":"Absent"},{"setting":{"Key":{"hive":"CurrentUser","key":"SOFTWARE\\Example"}},"value":{"Dword":1}}],"applier":"User","takes":"Now"},
] satisfies Advice[];

export const settingValueSamples = [
  {"setting":{"Dword":{"hive":"LocalMachine","key":"SOFTWARE\\Example","name":"Enabled"}},"value":"Absent"},
  {"setting":{"Key":{"hive":"CurrentUser","key":"SOFTWARE\\Example"}},"value":{"Dword":1}},
  {"setting":{"Service":{"name":"Example"}},"value":"Present"},
  {"setting":{"PowerAc":{"subgroup":"2a737441-1930-4402-8d77-b2bebba30001","setting":"2a737441-1930-4402-8d77-b2bebba30002","scheme":"2a737441-1930-4402-8d77-b2bebba30003"}},"value":{"Service":{"start":2,"delayed":true,"running":false}}},
  {"setting":"ActiveScheme","value":{"Index":1}},
  {"setting":"Hibernation","value":{"Scheme":"2a737441-1930-4402-8d77-b2bebba30004"}},
  {"setting":{"StartupApproval":{"hive":"CurrentUser","list":"Run","name":"Example"}},"value":{"On":true}},
  {"setting":{"PowerAc":{"subgroup":"2a737441-1930-4402-8d77-b2bebba30001","setting":"2a737441-1930-4402-8d77-b2bebba30002","scheme":null}},"value":{"Bytes":[1,2,3]}},
  {"setting":{"StartupApproval":{"hive":"LocalMachine","list":"Run","name":"Example"}},"value":"Absent"},
  {"setting":{"StartupApproval":{"hive":"LocalMachine","list":"Run32","name":"Example"}},"value":{"Dword":1}},
  {"setting":{"StartupApproval":{"hive":"LocalMachine","list":"StartupFolder","name":"Example"}},"value":"Present"},
] satisfies SettingValue[];

export const settingResultSamples = [
  {"change":{"setting":{"Dword":{"hive":"LocalMachine","key":"SOFTWARE\\Example","name":"Enabled"}},"from":"Absent","to":"Absent"},"error":5},
  {"change":{"setting":{"Dword":{"hive":"LocalMachine","key":"SOFTWARE\\Example","name":"Enabled"}},"from":"Absent","to":"Absent"},"error":null},
  {"change":{"setting":{"Key":{"hive":"CurrentUser","key":"SOFTWARE\\Example"}},"from":{"Dword":1},"to":"Absent"},"error":5},
  {"change":{"setting":{"Key":{"hive":"CurrentUser","key":"SOFTWARE\\Example"}},"from":{"Dword":1},"to":"Absent"},"error":null},
  {"change":{"setting":{"Service":{"name":"Example"}},"from":"Present","to":"Absent"},"error":5},
  {"change":{"setting":{"Service":{"name":"Example"}},"from":"Present","to":"Absent"},"error":null},
  {"change":{"setting":{"PowerAc":{"subgroup":"2a737441-1930-4402-8d77-b2bebba30001","setting":"2a737441-1930-4402-8d77-b2bebba30002","scheme":"2a737441-1930-4402-8d77-b2bebba30003"}},"from":{"Service":{"start":2,"delayed":true,"running":false}},"to":"Absent"},"error":5},
  {"change":{"setting":{"PowerAc":{"subgroup":"2a737441-1930-4402-8d77-b2bebba30001","setting":"2a737441-1930-4402-8d77-b2bebba30002","scheme":"2a737441-1930-4402-8d77-b2bebba30003"}},"from":{"Service":{"start":2,"delayed":true,"running":false}},"to":"Absent"},"error":null},
  {"change":{"setting":"ActiveScheme","from":{"Index":1},"to":"Absent"},"error":5},
  {"change":{"setting":"ActiveScheme","from":{"Index":1},"to":"Absent"},"error":null},
  {"change":{"setting":"Hibernation","from":{"Scheme":"2a737441-1930-4402-8d77-b2bebba30004"},"to":"Absent"},"error":5},
  {"change":{"setting":"Hibernation","from":{"Scheme":"2a737441-1930-4402-8d77-b2bebba30004"},"to":"Absent"},"error":null},
  {"change":{"setting":{"StartupApproval":{"hive":"CurrentUser","list":"Run","name":"Example"}},"from":{"On":true},"to":"Absent"},"error":5},
  {"change":{"setting":{"StartupApproval":{"hive":"CurrentUser","list":"Run","name":"Example"}},"from":{"On":true},"to":"Absent"},"error":null},
  {"change":{"setting":{"PowerAc":{"subgroup":"2a737441-1930-4402-8d77-b2bebba30001","setting":"2a737441-1930-4402-8d77-b2bebba30002","scheme":null}},"from":{"Bytes":[1,2,3]},"to":"Absent"},"error":5},
  {"change":{"setting":{"PowerAc":{"subgroup":"2a737441-1930-4402-8d77-b2bebba30001","setting":"2a737441-1930-4402-8d77-b2bebba30002","scheme":null}},"from":{"Bytes":[1,2,3]},"to":"Absent"},"error":null},
  {"change":{"setting":{"StartupApproval":{"hive":"LocalMachine","list":"Run","name":"Example"}},"from":"Absent","to":"Absent"},"error":5},
  {"change":{"setting":{"StartupApproval":{"hive":"LocalMachine","list":"Run","name":"Example"}},"from":"Absent","to":"Absent"},"error":null},
  {"change":{"setting":{"StartupApproval":{"hive":"LocalMachine","list":"Run32","name":"Example"}},"from":{"Dword":1},"to":"Absent"},"error":5},
  {"change":{"setting":{"StartupApproval":{"hive":"LocalMachine","list":"Run32","name":"Example"}},"from":{"Dword":1},"to":"Absent"},"error":null},
  {"change":{"setting":{"StartupApproval":{"hive":"LocalMachine","list":"StartupFolder","name":"Example"}},"from":"Present","to":"Absent"},"error":5},
  {"change":{"setting":{"StartupApproval":{"hive":"LocalMachine","list":"StartupFolder","name":"Example"}},"from":"Present","to":"Absent"},"error":null},
] satisfies SettingResult[];

export const restorePointSamples = [
  "Created",
  {"Reused":{"at_ms":1790000000000}},
  "Off",
  {"Failed":{"error":1460}},
  "Unconfirmed",
  {"Reused":{"at_ms":null}},
] satisfies RestorePoint[];

export const tweakChangeSamples = [
  {"tweak":"FastStartup","subject":"Example","undo_of":3,"results":[{"change":{"setting":{"Dword":{"hive":"LocalMachine","key":"SOFTWARE\\Example","name":"Enabled"}},"from":"Absent","to":"Absent"},"error":5},{"change":{"setting":{"Key":{"hive":"CurrentUser","key":"SOFTWARE\\Example"}},"from":{"Dword":1},"to":"Absent"},"error":null}],"restore_point":"Created","journal":12},
  {"tweak":"Hibernation","subject":"Example","undo_of":3,"results":[{"change":{"setting":{"Dword":{"hive":"LocalMachine","key":"SOFTWARE\\Example","name":"Enabled"}},"from":"Absent","to":"Absent"},"error":5},{"change":{"setting":{"Key":{"hive":"CurrentUser","key":"SOFTWARE\\Example"}},"from":{"Dword":1},"to":"Absent"},"error":null}],"restore_point":{"Reused":{"at_ms":1790000000000}},"journal":12},
  {"tweak":"PowerPlan","subject":"Example","undo_of":3,"results":[{"change":{"setting":{"Dword":{"hive":"LocalMachine","key":"SOFTWARE\\Example","name":"Enabled"}},"from":"Absent","to":"Absent"},"error":5},{"change":{"setting":{"Key":{"hive":"CurrentUser","key":"SOFTWARE\\Example"}},"from":{"Dword":1},"to":"Absent"},"error":null}],"restore_point":"Off","journal":12},
  {"tweak":"PcieAspm","subject":"Example","undo_of":3,"results":[{"change":{"setting":{"Dword":{"hive":"LocalMachine","key":"SOFTWARE\\Example","name":"Enabled"}},"from":"Absent","to":"Absent"},"error":5},{"change":{"setting":{"Key":{"hive":"CurrentUser","key":"SOFTWARE\\Example"}},"from":{"Dword":1},"to":"Absent"},"error":null}],"restore_point":{"Failed":{"error":1460}},"journal":12},
  {"tweak":"UsbSuspend","subject":"Example","undo_of":3,"results":[{"change":{"setting":{"Dword":{"hive":"LocalMachine","key":"SOFTWARE\\Example","name":"Enabled"}},"from":"Absent","to":"Absent"},"error":5},{"change":{"setting":{"Key":{"hive":"CurrentUser","key":"SOFTWARE\\Example"}},"from":{"Dword":1},"to":"Absent"},"error":null}],"restore_point":"Unconfirmed","journal":12},
  {"tweak":"SysMain","subject":"Example","undo_of":3,"results":[{"change":{"setting":{"Dword":{"hive":"LocalMachine","key":"SOFTWARE\\Example","name":"Enabled"}},"from":"Absent","to":"Absent"},"error":5},{"change":{"setting":{"Key":{"hive":"CurrentUser","key":"SOFTWARE\\Example"}},"from":{"Dword":1},"to":"Absent"},"error":null}],"restore_point":{"Reused":{"at_ms":null}},"journal":12},
  {"tweak":"WindowsSearch","subject":null,"undo_of":null,"results":[{"change":{"setting":{"Dword":{"hive":"LocalMachine","key":"SOFTWARE\\Example","name":"Enabled"}},"from":"Absent","to":"Absent"},"error":5},{"change":{"setting":{"Key":{"hive":"CurrentUser","key":"SOFTWARE\\Example"}},"from":{"Dword":1},"to":"Absent"},"error":null}],"restore_point":null,"journal":null},
  {"tweak":"FastStartup","subject":null,"undo_of":null,"results":[],"restore_point":null,"journal":null},
] satisfies TweakChange[];

export const helpedSamples = [
  {"days":7,"before":9,"after":2},
] satisfies Helped[];

export const tweakRowSamples = [
  {"id":1,"applied_ms":1790000000000,"undone_ms":1790000100000,"applier":"User","change":{"tweak":"FastStartup","subject":"Example","undo_of":3,"results":[{"change":{"setting":{"Dword":{"hive":"LocalMachine","key":"SOFTWARE\\Example","name":"Enabled"}},"from":"Absent","to":"Absent"},"error":5},{"change":{"setting":{"Key":{"hive":"CurrentUser","key":"SOFTWARE\\Example"}},"from":{"Dword":1},"to":"Absent"},"error":null}],"restore_point":"Created","journal":12},"helped":{"days":7,"before":9,"after":2},"outside":false},
  {"id":2,"applied_ms":1790000000000,"undone_ms":null,"applier":"Elevated","change":{"tweak":"Hibernation","subject":"Example","undo_of":3,"results":[{"change":{"setting":{"Dword":{"hive":"LocalMachine","key":"SOFTWARE\\Example","name":"Enabled"}},"from":"Absent","to":"Absent"},"error":5},{"change":{"setting":{"Key":{"hive":"CurrentUser","key":"SOFTWARE\\Example"}},"from":{"Dword":1},"to":"Absent"},"error":null}],"restore_point":{"Reused":{"at_ms":1790000000000}},"journal":12},"helped":null,"outside":true},
  {"id":3,"applied_ms":1790000000000,"undone_ms":1790000100000,"applier":"User","change":{"tweak":"PowerPlan","subject":"Example","undo_of":3,"results":[{"change":{"setting":{"Dword":{"hive":"LocalMachine","key":"SOFTWARE\\Example","name":"Enabled"}},"from":"Absent","to":"Absent"},"error":5},{"change":{"setting":{"Key":{"hive":"CurrentUser","key":"SOFTWARE\\Example"}},"from":{"Dword":1},"to":"Absent"},"error":null}],"restore_point":"Off","journal":12},"helped":{"days":7,"before":9,"after":2},"outside":false},
  {"id":4,"applied_ms":1790000000000,"undone_ms":null,"applier":"Elevated","change":{"tweak":"PcieAspm","subject":"Example","undo_of":3,"results":[{"change":{"setting":{"Dword":{"hive":"LocalMachine","key":"SOFTWARE\\Example","name":"Enabled"}},"from":"Absent","to":"Absent"},"error":5},{"change":{"setting":{"Key":{"hive":"CurrentUser","key":"SOFTWARE\\Example"}},"from":{"Dword":1},"to":"Absent"},"error":null}],"restore_point":{"Failed":{"error":1460}},"journal":12},"helped":null,"outside":true},
  {"id":5,"applied_ms":1790000000000,"undone_ms":1790000100000,"applier":"User","change":{"tweak":"UsbSuspend","subject":"Example","undo_of":3,"results":[{"change":{"setting":{"Dword":{"hive":"LocalMachine","key":"SOFTWARE\\Example","name":"Enabled"}},"from":"Absent","to":"Absent"},"error":5},{"change":{"setting":{"Key":{"hive":"CurrentUser","key":"SOFTWARE\\Example"}},"from":{"Dword":1},"to":"Absent"},"error":null}],"restore_point":"Unconfirmed","journal":12},"helped":{"days":7,"before":9,"after":2},"outside":false},
  {"id":6,"applied_ms":1790000000000,"undone_ms":null,"applier":"Elevated","change":{"tweak":"SysMain","subject":"Example","undo_of":3,"results":[{"change":{"setting":{"Dword":{"hive":"LocalMachine","key":"SOFTWARE\\Example","name":"Enabled"}},"from":"Absent","to":"Absent"},"error":5},{"change":{"setting":{"Key":{"hive":"CurrentUser","key":"SOFTWARE\\Example"}},"from":{"Dword":1},"to":"Absent"},"error":null}],"restore_point":{"Reused":{"at_ms":null}},"journal":12},"helped":null,"outside":true},
  {"id":7,"applied_ms":1790000000000,"undone_ms":1790000100000,"applier":"User","change":{"tweak":"WindowsSearch","subject":null,"undo_of":null,"results":[{"change":{"setting":{"Dword":{"hive":"LocalMachine","key":"SOFTWARE\\Example","name":"Enabled"}},"from":"Absent","to":"Absent"},"error":5},{"change":{"setting":{"Key":{"hive":"CurrentUser","key":"SOFTWARE\\Example"}},"from":{"Dword":1},"to":"Absent"},"error":null}],"restore_point":null,"journal":null},"helped":{"days":7,"before":9,"after":2},"outside":false},
  {"id":8,"applied_ms":1790000000000,"undone_ms":null,"applier":"Elevated","change":{"tweak":"FastStartup","subject":null,"undo_of":null,"results":[],"restore_point":null,"journal":null},"helped":null,"outside":true},
] satisfies TweakRow[];

export const tweaksDtoSamples = [
  {"advice":[{"tweak":"FastStartup","subject":"Example","status":"Ready","signals":[{"Finding":"SlowBoot"},{"UnfinishedShutdowns":{"count":3}},{"SleepFalls":{"count":2}}],"changes":[{"setting":{"Dword":{"hive":"LocalMachine","key":"SOFTWARE\\Example","name":"Enabled"}},"from":"Absent","to":"Absent"},{"setting":{"Key":{"hive":"CurrentUser","key":"SOFTWARE\\Example"}},"from":{"Dword":1},"to":"Absent"}],"current":[{"setting":{"Dword":{"hive":"LocalMachine","key":"SOFTWARE\\Example","name":"Enabled"}},"value":"Absent"},{"setting":{"Key":{"hive":"CurrentUser","key":"SOFTWARE\\Example"}},"value":{"Dword":1}}],"applier":"User","takes":"Now"},{"tweak":"Hibernation","subject":"Example","status":"AlreadySet","signals":[{"Finding":"SlowBoot"},{"UnfinishedShutdowns":{"count":3}},{"SleepFalls":{"count":2}}],"changes":[{"setting":{"Dword":{"hive":"LocalMachine","key":"SOFTWARE\\Example","name":"Enabled"}},"from":"Absent","to":"Absent"},{"setting":{"Key":{"hive":"CurrentUser","key":"SOFTWARE\\Example"}},"from":{"Dword":1},"to":"Absent"}],"current":[{"setting":{"Dword":{"hive":"LocalMachine","key":"SOFTWARE\\Example","name":"Enabled"}},"value":"Absent"},{"setting":{"Key":{"hive":"CurrentUser","key":"SOFTWARE\\Example"}},"value":{"Dword":1}}],"applier":"Elevated","takes":"NextShutdown"}],"history":[{"id":1,"applied_ms":1790000000000,"undone_ms":1790000100000,"applier":"User","change":{"tweak":"FastStartup","subject":"Example","undo_of":3,"results":[{"change":{"setting":{"Dword":{"hive":"LocalMachine","key":"SOFTWARE\\Example","name":"Enabled"}},"from":"Absent","to":"Absent"},"error":5},{"change":{"setting":{"Key":{"hive":"CurrentUser","key":"SOFTWARE\\Example"}},"from":{"Dword":1},"to":"Absent"},"error":null}],"restore_point":"Created","journal":12},"helped":{"days":7,"before":9,"after":2},"outside":false},{"id":2,"applied_ms":1790000000000,"undone_ms":null,"applier":"Elevated","change":{"tweak":"Hibernation","subject":"Example","undo_of":3,"results":[{"change":{"setting":{"Dword":{"hive":"LocalMachine","key":"SOFTWARE\\Example","name":"Enabled"}},"from":"Absent","to":"Absent"},"error":5},{"change":{"setting":{"Key":{"hive":"CurrentUser","key":"SOFTWARE\\Example"}},"from":{"Dword":1},"to":"Absent"},"error":null}],"restore_point":{"Reused":{"at_ms":1790000000000}},"journal":12},"helped":null,"outside":true}],"last_restore_point_ms":1790000000000},
  {"advice":[{"tweak":"FastStartup","subject":null,"status":"Ready","signals":[],"changes":[{"setting":{"Dword":{"hive":"LocalMachine","key":"SOFTWARE\\Example","name":"Enabled"}},"from":"Absent","to":"Absent"},{"setting":{"Key":{"hive":"CurrentUser","key":"SOFTWARE\\Example"}},"from":{"Dword":1},"to":"Absent"}],"current":[{"setting":{"Dword":{"hive":"LocalMachine","key":"SOFTWARE\\Example","name":"Enabled"}},"value":"Absent"},{"setting":{"Key":{"hive":"CurrentUser","key":"SOFTWARE\\Example"}},"value":{"Dword":1}}],"applier":"User","takes":"Now"},{"tweak":"Hibernation","subject":null,"status":"AlreadySet","signals":[],"changes":[{"setting":{"Dword":{"hive":"LocalMachine","key":"SOFTWARE\\Example","name":"Enabled"}},"from":"Absent","to":"Absent"},{"setting":{"Key":{"hive":"CurrentUser","key":"SOFTWARE\\Example"}},"from":{"Dword":1},"to":"Absent"}],"current":[{"setting":{"Dword":{"hive":"LocalMachine","key":"SOFTWARE\\Example","name":"Enabled"}},"value":"Absent"},{"setting":{"Key":{"hive":"CurrentUser","key":"SOFTWARE\\Example"}},"value":{"Dword":1}}],"applier":"Elevated","takes":"NextShutdown"}],"history":[{"id":1,"applied_ms":1790000000000,"undone_ms":1790000100000,"applier":"User","change":{"tweak":"FastStartup","subject":"Example","undo_of":3,"results":[{"change":{"setting":{"Dword":{"hive":"LocalMachine","key":"SOFTWARE\\Example","name":"Enabled"}},"from":"Absent","to":"Absent"},"error":5},{"change":{"setting":{"Key":{"hive":"CurrentUser","key":"SOFTWARE\\Example"}},"from":{"Dword":1},"to":"Absent"},"error":null}],"restore_point":"Created","journal":12},"helped":{"days":7,"before":9,"after":2},"outside":false},{"id":2,"applied_ms":1790000000000,"undone_ms":null,"applier":"Elevated","change":{"tweak":"Hibernation","subject":"Example","undo_of":3,"results":[{"change":{"setting":{"Dword":{"hive":"LocalMachine","key":"SOFTWARE\\Example","name":"Enabled"}},"from":"Absent","to":"Absent"},"error":5},{"change":{"setting":{"Key":{"hive":"CurrentUser","key":"SOFTWARE\\Example"}},"from":{"Dword":1},"to":"Absent"},"error":null}],"restore_point":{"Reused":{"at_ms":1790000000000}},"journal":12},"helped":null,"outside":true}],"last_restore_point_ms":null},
] satisfies TweaksDto[];

export const tweaksAppliedSamples = [
  {"ids":[3,4],"exit_code":0},
  {"ids":[3,4],"exit_code":null},
] satisfies TweaksApplied[];

export const nvmeDetailSamples = [
  {"health":{"critical_warning":0,"temperature_c":41,"available_spare_pct":100,"spare_threshold_pct":10,"percentage_used":3,"media_errors":0},"data_read_bytes":5000000000,"data_written_bytes":4000000000,"power_cycles":300,"power_on_hours":4000,"unsafe_shutdowns":7,"error_log_entries":0},
] satisfies NvmeDetail[];

export const runAsSamples = [
  "System",
  "LocalService",
  "NetworkService",
  {"CurrentUser":{"elevated":true}},
  "OtherUser",
  "Unknown",
  {"CurrentUser":{"elevated":false}},
] satisfies RunAs[];

export const signatureSamples = [
  {"Valid":{"signer":"Example Corp"}},
  "Invalid",
  "Unsigned",
  "Unknown",
] satisfies Signature[];

export const processFactsSamples = [
  {"started_ms":1790000000000,"threads":12,"processes":2,"path":"C:\\Example\\example.exe","run_as":"System","signature":{"Valid":{"signer":"Example Corp"}}},
  {"started_ms":1790000000000,"threads":12,"processes":2,"path":"C:\\Example\\example.exe","run_as":"LocalService","signature":"Invalid"},
  {"started_ms":1790000000000,"threads":12,"processes":2,"path":"C:\\Example\\example.exe","run_as":"NetworkService","signature":"Unsigned"},
  {"started_ms":1790000000000,"threads":12,"processes":2,"path":"C:\\Example\\example.exe","run_as":{"CurrentUser":{"elevated":true}},"signature":"Unknown"},
  {"started_ms":1790000000000,"threads":12,"processes":2,"path":"C:\\Example\\example.exe","run_as":"OtherUser","signature":{"Valid":{"signer":"Example Corp"}}},
  {"started_ms":1790000000000,"threads":12,"processes":2,"path":"C:\\Example\\example.exe","run_as":"Unknown","signature":"Invalid"},
  {"started_ms":1790000000000,"threads":12,"processes":2,"path":"C:\\Example\\example.exe","run_as":{"CurrentUser":{"elevated":false}},"signature":"Unsigned"},
  {"started_ms":null,"threads":null,"processes":null,"path":null,"run_as":"Unknown","signature":"Unknown"},
] satisfies ProcessFacts[];

export const programMarksSamples = [
  {"occurrences":[{"id":1,"kind":"Freeze","start_ms":1790000000000,"end_ms":1790000005000,"severity":"Light","cause":"Unclear"},{"id":2,"kind":"Stutter","start_ms":1790000000000,"end_ms":1790000005000,"severity":"Medium","cause":"Unclear"}],"episodes":[{"kind":"MemoryHigh","start_ms":1790000000000,"end_ms":1790000600000,"peak":97.5,"culprit":{"pid":4120,"name":"example.exe","value":1500000000.0},"truncated":true},{"kind":"MemoryHigh","start_ms":1790000000000,"end_ms":1790000600000,"peak":97.5,"culprit":null,"truncated":false}]},
] satisfies ProgramMarks[];

export const programPointSamples = [
  {"ts_ms":1790000000000,"cpu_pct":14.5,"private_bytes":123456789,"io_bps":4096.0},
] satisfies ProgramPoint[];

export const smartPointSamples = [
  {"read_ms":1790000000000,"current":100,"worst":99,"raw":12},
] satisfies SmartPoint[];

export const diskHealthSamples = [
  {"name":"0 C:","read_ms":1790000000000,"predict_failure":true,"attributes":[{"id":5,"current":100,"worst":99,"raw":12}],"nvme":{"critical_warning":0,"temperature_c":41,"available_spare_pct":100,"spare_threshold_pct":10,"percentage_used":3,"media_errors":0}},
  {"name":"0 C:","read_ms":1790000000000,"predict_failure":false,"attributes":[{"id":5,"current":100,"worst":99,"raw":12}],"nvme":null},
] satisfies DiskHealth[];

export const testKindSamples = [
  "MemoryQuick",
  "DiskSurface",
  "VolumeCheck",
  "DiskBenchmark",
] satisfies TestKind[];

export const testOutcomeSamples = [
  "Clean",
  "Problems",
  "Interrupted",
  "Failed",
] satisfies TestOutcome[];

export const patternSamples = [
  "OwnAddress",
  "Random",
  "Compare",
  "SolidBits",
  "Checkerboard",
  "WalkingOnes",
  "WalkingZeros",
] satisfies Pattern[];

export const memoryErrorSamples = [
  {"offset":4096,"pattern":"OwnAddress","expected":255,"actual":254},
  {"offset":4096,"pattern":"Random","expected":255,"actual":254},
  {"offset":4096,"pattern":"Compare","expected":255,"actual":254},
  {"offset":4096,"pattern":"SolidBits","expected":255,"actual":254},
  {"offset":4096,"pattern":"Checkerboard","expected":255,"actual":254},
  {"offset":4096,"pattern":"WalkingOnes","expected":255,"actual":254},
  {"offset":4096,"pattern":"WalkingZeros","expected":255,"actual":254},
] satisfies MemoryError[];

export const memoryQuickSamples = [
  {"total_bytes":17179869184,"tested_bytes":2000000000,"rounds":2,"passes":3,"error_count":1,"errors":[{"offset":4096,"pattern":"OwnAddress","expected":255,"actual":254},{"offset":4096,"pattern":"Random","expected":255,"actual":254},{"offset":4096,"pattern":"Compare","expected":255,"actual":254},{"offset":4096,"pattern":"SolidBits","expected":255,"actual":254},{"offset":4096,"pattern":"Checkerboard","expected":255,"actual":254},{"offset":4096,"pattern":"WalkingOnes","expected":255,"actual":254},{"offset":4096,"pattern":"WalkingZeros","expected":255,"actual":254}],"failure":"out of memory"},
  {"total_bytes":17179869184,"tested_bytes":2000000000,"rounds":2,"passes":3,"error_count":1,"errors":[{"offset":4096,"pattern":"OwnAddress","expected":255,"actual":254},{"offset":4096,"pattern":"Random","expected":255,"actual":254},{"offset":4096,"pattern":"Compare","expected":255,"actual":254},{"offset":4096,"pattern":"SolidBits","expected":255,"actual":254},{"offset":4096,"pattern":"Checkerboard","expected":255,"actual":254},{"offset":4096,"pattern":"WalkingOnes","expected":255,"actual":254},{"offset":4096,"pattern":"WalkingZeros","expected":255,"actual":254}],"failure":null},
] satisfies MemoryQuick[];

export const regionSamples = [
  "Unread",
  {"Read":{"worst_ms":12.5}},
  "Unreadable",
] satisfies Region[];

export const slowBlockSamples = [
  {"offset":4096,"ms":480.5},
] satisfies SlowBlock[];

export const surfaceScanSamples = [
  {"disk":0,"media":"Hdd","disk_bytes":1000000000000,"sector_bytes":512,"from_bytes":0,"to_bytes":1000000000000,"bands":[4,4,4,4,4,4],"unreadable_blocks":2,"regions":["Unread",{"Read":{"worst_ms":12.5}},"Unreadable"],"worst":[{"offset":4096,"ms":480.5}],"unreadable":[8192],"failure":"read failed","outside_io":true},
  {"disk":0,"media":"Hdd","disk_bytes":1000000000000,"sector_bytes":512,"from_bytes":0,"to_bytes":1000000000000,"bands":[4,4,4,4,4,4],"unreadable_blocks":2,"regions":["Unread",{"Read":{"worst_ms":12.5}},"Unreadable"],"worst":[{"offset":4096,"ms":480.5}],"unreadable":[8192],"failure":null,"outside_io":false},
] satisfies SurfaceScan[];

export const busSamples = [
  "Sata",
  "Nvme",
  "Usb",
  "Other",
] satisfies Bus[];

export const benchModeSamples = [
  "SeqRead",
  "SeqWrite",
  "RandRead32",
  "RandWrite32",
  "RandRead1",
  "RandWrite1",
] satisfies BenchMode[];

export const benchResultSamples = [
  {"mode":"SeqRead","mb_per_s":1500.5,"iops":90000.5,"max_ms":12.5},
  {"mode":"SeqWrite","mb_per_s":1500.5,"iops":90000.5,"max_ms":12.5},
  {"mode":"RandRead32","mb_per_s":1500.5,"iops":90000.5,"max_ms":12.5},
  {"mode":"RandWrite32","mb_per_s":1500.5,"iops":90000.5,"max_ms":12.5},
  {"mode":"RandRead1","mb_per_s":1500.5,"iops":90000.5,"max_ms":12.5},
  {"mode":"RandWrite1","mb_per_s":1500.5,"iops":90000.5,"max_ms":12.5},
] satisfies BenchResult[];

export const benchmarkSamples = [
  {"volume":"C:\\","bus":"Nvme","media":"Ssd","file_bytes":1000000000,"written_bytes":2000000000,"results":[{"mode":"SeqRead","mb_per_s":1500.5,"iops":90000.5,"max_ms":12.5},{"mode":"SeqWrite","mb_per_s":1500.5,"iops":90000.5,"max_ms":12.5},{"mode":"RandRead32","mb_per_s":1500.5,"iops":90000.5,"max_ms":12.5},{"mode":"RandWrite32","mb_per_s":1500.5,"iops":90000.5,"max_ms":12.5},{"mode":"RandRead1","mb_per_s":1500.5,"iops":90000.5,"max_ms":12.5},{"mode":"RandWrite1","mb_per_s":1500.5,"iops":90000.5,"max_ms":12.5}],"outside_io":true,"failure":"no room"},
  {"volume":"C:\\","bus":"Nvme","media":"Ssd","file_bytes":1000000000,"written_bytes":2000000000,"results":[{"mode":"SeqRead","mb_per_s":1500.5,"iops":90000.5,"max_ms":12.5},{"mode":"SeqWrite","mb_per_s":1500.5,"iops":90000.5,"max_ms":12.5},{"mode":"RandRead32","mb_per_s":1500.5,"iops":90000.5,"max_ms":12.5},{"mode":"RandWrite32","mb_per_s":1500.5,"iops":90000.5,"max_ms":12.5},{"mode":"RandRead1","mb_per_s":1500.5,"iops":90000.5,"max_ms":12.5},{"mode":"RandWrite1","mb_per_s":1500.5,"iops":90000.5,"max_ms":12.5}],"outside_io":false,"failure":null},
] satisfies Benchmark[];

export const volumeResultSamples = [
  "NoProblems",
  "ProblemsFound",
  "CouldNotCheck",
] satisfies VolumeResult[];

export const volumeCheckSamples = [
  {"volume":"C:\\","exit_code":0,"result":"NoProblems","output":"The file system is fine.","dirty_before":false,"dirty_after":true},
  {"volume":"C:\\","exit_code":null,"result":"NoProblems","output":"The file system is fine.","dirty_before":null,"dirty_after":null},
  {"volume":"C:\\","exit_code":0,"result":"ProblemsFound","output":"The file system is fine.","dirty_before":false,"dirty_after":true},
  {"volume":"C:\\","exit_code":null,"result":"ProblemsFound","output":"The file system is fine.","dirty_before":null,"dirty_after":null},
  {"volume":"C:\\","exit_code":0,"result":"CouldNotCheck","output":"The file system is fine.","dirty_before":false,"dirty_after":true},
  {"volume":"C:\\","exit_code":null,"result":"CouldNotCheck","output":"The file system is fine.","dirty_before":null,"dirty_after":null},
] satisfies VolumeCheck[];

export const testDataSamples = [
  {"MemoryQuick":{"total_bytes":17179869184,"tested_bytes":2000000000,"rounds":2,"passes":3,"error_count":1,"errors":[{"offset":4096,"pattern":"OwnAddress","expected":255,"actual":254},{"offset":4096,"pattern":"Random","expected":255,"actual":254},{"offset":4096,"pattern":"Compare","expected":255,"actual":254},{"offset":4096,"pattern":"SolidBits","expected":255,"actual":254},{"offset":4096,"pattern":"Checkerboard","expected":255,"actual":254},{"offset":4096,"pattern":"WalkingOnes","expected":255,"actual":254},{"offset":4096,"pattern":"WalkingZeros","expected":255,"actual":254}],"failure":"out of memory"}},
  {"DiskSurface":{"disk":0,"media":"Hdd","disk_bytes":1000000000000,"sector_bytes":512,"from_bytes":0,"to_bytes":1000000000000,"bands":[4,4,4,4,4,4],"unreadable_blocks":2,"regions":["Unread",{"Read":{"worst_ms":12.5}},"Unreadable"],"worst":[{"offset":4096,"ms":480.5}],"unreadable":[8192],"failure":"read failed","outside_io":true}},
  {"VolumeCheck":{"volume":"C:\\","exit_code":0,"result":"ProblemsFound","output":"The file system is fine.","dirty_before":false,"dirty_after":true}},
  {"DiskBenchmark":{"volume":"C:\\","bus":"Nvme","media":"Ssd","file_bytes":1000000000,"written_bytes":2000000000,"results":[{"mode":"SeqRead","mb_per_s":1500.5,"iops":90000.5,"max_ms":12.5},{"mode":"SeqWrite","mb_per_s":1500.5,"iops":90000.5,"max_ms":12.5},{"mode":"RandRead32","mb_per_s":1500.5,"iops":90000.5,"max_ms":12.5},{"mode":"RandWrite32","mb_per_s":1500.5,"iops":90000.5,"max_ms":12.5},{"mode":"RandRead1","mb_per_s":1500.5,"iops":90000.5,"max_ms":12.5},{"mode":"RandWrite1","mb_per_s":1500.5,"iops":90000.5,"max_ms":12.5}],"outside_io":true,"failure":"no room"}},
  {"MemoryQuick":{"total_bytes":17179869184,"tested_bytes":2000000000,"rounds":2,"passes":3,"error_count":1,"errors":[{"offset":4096,"pattern":"OwnAddress","expected":255,"actual":254},{"offset":4096,"pattern":"Random","expected":255,"actual":254},{"offset":4096,"pattern":"Compare","expected":255,"actual":254},{"offset":4096,"pattern":"SolidBits","expected":255,"actual":254},{"offset":4096,"pattern":"Checkerboard","expected":255,"actual":254},{"offset":4096,"pattern":"WalkingOnes","expected":255,"actual":254},{"offset":4096,"pattern":"WalkingZeros","expected":255,"actual":254}],"failure":null}},
  {"DiskSurface":{"disk":0,"media":"Hdd","disk_bytes":1000000000000,"sector_bytes":512,"from_bytes":0,"to_bytes":1000000000000,"bands":[4,4,4,4,4,4],"unreadable_blocks":2,"regions":["Unread",{"Read":{"worst_ms":12.5}},"Unreadable"],"worst":[{"offset":4096,"ms":480.5}],"unreadable":[8192],"failure":null,"outside_io":false}},
  {"DiskBenchmark":{"volume":"C:\\","bus":"Nvme","media":"Ssd","file_bytes":1000000000,"written_bytes":2000000000,"results":[{"mode":"SeqRead","mb_per_s":1500.5,"iops":90000.5,"max_ms":12.5},{"mode":"SeqWrite","mb_per_s":1500.5,"iops":90000.5,"max_ms":12.5},{"mode":"RandRead32","mb_per_s":1500.5,"iops":90000.5,"max_ms":12.5},{"mode":"RandWrite32","mb_per_s":1500.5,"iops":90000.5,"max_ms":12.5},{"mode":"RandRead1","mb_per_s":1500.5,"iops":90000.5,"max_ms":12.5},{"mode":"RandWrite1","mb_per_s":1500.5,"iops":90000.5,"max_ms":12.5}],"outside_io":false,"failure":null}},
  {"VolumeCheck":{"volume":"C:\\","exit_code":0,"result":"NoProblems","output":"The file system is fine.","dirty_before":false,"dirty_after":true}},
  {"VolumeCheck":{"volume":"C:\\","exit_code":null,"result":"NoProblems","output":"The file system is fine.","dirty_before":null,"dirty_after":null}},
  {"VolumeCheck":{"volume":"C:\\","exit_code":0,"result":"ProblemsFound","output":"The file system is fine.","dirty_before":false,"dirty_after":true}},
  {"VolumeCheck":{"volume":"C:\\","exit_code":null,"result":"ProblemsFound","output":"The file system is fine.","dirty_before":null,"dirty_after":null}},
  {"VolumeCheck":{"volume":"C:\\","exit_code":0,"result":"CouldNotCheck","output":"The file system is fine.","dirty_before":false,"dirty_after":true}},
  {"VolumeCheck":{"volume":"C:\\","exit_code":null,"result":"CouldNotCheck","output":"The file system is fine.","dirty_before":null,"dirty_after":null}},
] satisfies TestData[];

export const testRunSamples = [
  {"subject":"Pc","started_ms":1790000000000,"ended_ms":1790000600000,"outcome":"Clean","data":{"MemoryQuick":{"total_bytes":17179869184,"tested_bytes":2000000000,"rounds":2,"passes":3,"error_count":1,"errors":[{"offset":4096,"pattern":"OwnAddress","expected":255,"actual":254},{"offset":4096,"pattern":"Random","expected":255,"actual":254},{"offset":4096,"pattern":"Compare","expected":255,"actual":254},{"offset":4096,"pattern":"SolidBits","expected":255,"actual":254},{"offset":4096,"pattern":"Checkerboard","expected":255,"actual":254},{"offset":4096,"pattern":"WalkingOnes","expected":255,"actual":254},{"offset":4096,"pattern":"WalkingZeros","expected":255,"actual":254}],"failure":"out of memory"}}},
  {"subject":{"Disk":"0 C:"},"started_ms":1790000000000,"ended_ms":1790000600000,"outcome":"Problems","data":{"DiskSurface":{"disk":0,"media":"Hdd","disk_bytes":1000000000000,"sector_bytes":512,"from_bytes":0,"to_bytes":1000000000000,"bands":[4,4,4,4,4,4],"unreadable_blocks":2,"regions":["Unread",{"Read":{"worst_ms":12.5}},"Unreadable"],"worst":[{"offset":4096,"ms":480.5}],"unreadable":[8192],"failure":"read failed","outside_io":true}}},
  {"subject":{"Volume":"C:\\"},"started_ms":1790000000000,"ended_ms":1790000600000,"outcome":"Interrupted","data":{"VolumeCheck":{"volume":"C:\\","exit_code":0,"result":"ProblemsFound","output":"The file system is fine.","dirty_before":false,"dirty_after":true}}},
  {"subject":"Pc","started_ms":1790000000000,"ended_ms":1790000600000,"outcome":"Failed","data":{"DiskBenchmark":{"volume":"C:\\","bus":"Nvme","media":"Ssd","file_bytes":1000000000,"written_bytes":2000000000,"results":[{"mode":"SeqRead","mb_per_s":1500.5,"iops":90000.5,"max_ms":12.5},{"mode":"SeqWrite","mb_per_s":1500.5,"iops":90000.5,"max_ms":12.5},{"mode":"RandRead32","mb_per_s":1500.5,"iops":90000.5,"max_ms":12.5},{"mode":"RandWrite32","mb_per_s":1500.5,"iops":90000.5,"max_ms":12.5},{"mode":"RandRead1","mb_per_s":1500.5,"iops":90000.5,"max_ms":12.5},{"mode":"RandWrite1","mb_per_s":1500.5,"iops":90000.5,"max_ms":12.5}],"outside_io":true,"failure":"no room"}}},
  {"subject":{"Disk":"0 C:"},"started_ms":1790000000000,"ended_ms":1790000600000,"outcome":"Clean","data":{"MemoryQuick":{"total_bytes":17179869184,"tested_bytes":2000000000,"rounds":2,"passes":3,"error_count":1,"errors":[{"offset":4096,"pattern":"OwnAddress","expected":255,"actual":254},{"offset":4096,"pattern":"Random","expected":255,"actual":254},{"offset":4096,"pattern":"Compare","expected":255,"actual":254},{"offset":4096,"pattern":"SolidBits","expected":255,"actual":254},{"offset":4096,"pattern":"Checkerboard","expected":255,"actual":254},{"offset":4096,"pattern":"WalkingOnes","expected":255,"actual":254},{"offset":4096,"pattern":"WalkingZeros","expected":255,"actual":254}],"failure":null}}},
  {"subject":{"Volume":"C:\\"},"started_ms":1790000000000,"ended_ms":1790000600000,"outcome":"Problems","data":{"DiskSurface":{"disk":0,"media":"Hdd","disk_bytes":1000000000000,"sector_bytes":512,"from_bytes":0,"to_bytes":1000000000000,"bands":[4,4,4,4,4,4],"unreadable_blocks":2,"regions":["Unread",{"Read":{"worst_ms":12.5}},"Unreadable"],"worst":[{"offset":4096,"ms":480.5}],"unreadable":[8192],"failure":null,"outside_io":false}}},
  {"subject":"Pc","started_ms":1790000000000,"ended_ms":1790000600000,"outcome":"Interrupted","data":{"DiskBenchmark":{"volume":"C:\\","bus":"Nvme","media":"Ssd","file_bytes":1000000000,"written_bytes":2000000000,"results":[{"mode":"SeqRead","mb_per_s":1500.5,"iops":90000.5,"max_ms":12.5},{"mode":"SeqWrite","mb_per_s":1500.5,"iops":90000.5,"max_ms":12.5},{"mode":"RandRead32","mb_per_s":1500.5,"iops":90000.5,"max_ms":12.5},{"mode":"RandWrite32","mb_per_s":1500.5,"iops":90000.5,"max_ms":12.5},{"mode":"RandRead1","mb_per_s":1500.5,"iops":90000.5,"max_ms":12.5},{"mode":"RandWrite1","mb_per_s":1500.5,"iops":90000.5,"max_ms":12.5}],"outside_io":false,"failure":null}}},
  {"subject":{"Disk":"0 C:"},"started_ms":1790000000000,"ended_ms":1790000600000,"outcome":"Failed","data":{"VolumeCheck":{"volume":"C:\\","exit_code":0,"result":"NoProblems","output":"The file system is fine.","dirty_before":false,"dirty_after":true}}},
  {"subject":{"Volume":"C:\\"},"started_ms":1790000000000,"ended_ms":1790000600000,"outcome":"Clean","data":{"VolumeCheck":{"volume":"C:\\","exit_code":null,"result":"NoProblems","output":"The file system is fine.","dirty_before":null,"dirty_after":null}}},
  {"subject":"Pc","started_ms":1790000000000,"ended_ms":1790000600000,"outcome":"Problems","data":{"VolumeCheck":{"volume":"C:\\","exit_code":0,"result":"ProblemsFound","output":"The file system is fine.","dirty_before":false,"dirty_after":true}}},
  {"subject":{"Disk":"0 C:"},"started_ms":1790000000000,"ended_ms":1790000600000,"outcome":"Interrupted","data":{"VolumeCheck":{"volume":"C:\\","exit_code":null,"result":"ProblemsFound","output":"The file system is fine.","dirty_before":null,"dirty_after":null}}},
  {"subject":{"Volume":"C:\\"},"started_ms":1790000000000,"ended_ms":1790000600000,"outcome":"Failed","data":{"VolumeCheck":{"volume":"C:\\","exit_code":0,"result":"CouldNotCheck","output":"The file system is fine.","dirty_before":false,"dirty_after":true}}},
  {"subject":"Pc","started_ms":1790000000000,"ended_ms":1790000600000,"outcome":"Clean","data":{"VolumeCheck":{"volume":"C:\\","exit_code":null,"result":"CouldNotCheck","output":"The file system is fine.","dirty_before":null,"dirty_after":null}}},
] satisfies TestRun[];

export const testRunDtoSamples = [
  {"id":1,"subject":"Pc","started_ms":1790000000000,"ended_ms":1790000600000,"outcome":"Clean","data":{"MemoryQuick":{"total_bytes":17179869184,"tested_bytes":2000000000,"rounds":2,"passes":3,"error_count":1,"errors":[{"offset":4096,"pattern":"OwnAddress","expected":255,"actual":254},{"offset":4096,"pattern":"Random","expected":255,"actual":254},{"offset":4096,"pattern":"Compare","expected":255,"actual":254},{"offset":4096,"pattern":"SolidBits","expected":255,"actual":254},{"offset":4096,"pattern":"Checkerboard","expected":255,"actual":254},{"offset":4096,"pattern":"WalkingOnes","expected":255,"actual":254},{"offset":4096,"pattern":"WalkingZeros","expected":255,"actual":254}],"failure":"out of memory"}}},
  {"id":2,"subject":{"Disk":"0 C:"},"started_ms":1790000000000,"ended_ms":1790000600000,"outcome":"Problems","data":{"DiskSurface":{"disk":0,"media":"Hdd","disk_bytes":1000000000000,"sector_bytes":512,"from_bytes":0,"to_bytes":1000000000000,"bands":[4,4,4,4,4,4],"unreadable_blocks":2,"regions":["Unread",{"Read":{"worst_ms":12.5}},"Unreadable"],"worst":[{"offset":4096,"ms":480.5}],"unreadable":[8192],"failure":"read failed","outside_io":true}}},
  {"id":3,"subject":{"Volume":"C:\\"},"started_ms":1790000000000,"ended_ms":1790000600000,"outcome":"Interrupted","data":{"VolumeCheck":{"volume":"C:\\","exit_code":0,"result":"ProblemsFound","output":"The file system is fine.","dirty_before":false,"dirty_after":true}}},
  {"id":4,"subject":"Pc","started_ms":1790000000000,"ended_ms":1790000600000,"outcome":"Failed","data":{"DiskBenchmark":{"volume":"C:\\","bus":"Nvme","media":"Ssd","file_bytes":1000000000,"written_bytes":2000000000,"results":[{"mode":"SeqRead","mb_per_s":1500.5,"iops":90000.5,"max_ms":12.5},{"mode":"SeqWrite","mb_per_s":1500.5,"iops":90000.5,"max_ms":12.5},{"mode":"RandRead32","mb_per_s":1500.5,"iops":90000.5,"max_ms":12.5},{"mode":"RandWrite32","mb_per_s":1500.5,"iops":90000.5,"max_ms":12.5},{"mode":"RandRead1","mb_per_s":1500.5,"iops":90000.5,"max_ms":12.5},{"mode":"RandWrite1","mb_per_s":1500.5,"iops":90000.5,"max_ms":12.5}],"outside_io":true,"failure":"no room"}}},
  {"id":5,"subject":{"Disk":"0 C:"},"started_ms":1790000000000,"ended_ms":1790000600000,"outcome":"Clean","data":{"MemoryQuick":{"total_bytes":17179869184,"tested_bytes":2000000000,"rounds":2,"passes":3,"error_count":1,"errors":[{"offset":4096,"pattern":"OwnAddress","expected":255,"actual":254},{"offset":4096,"pattern":"Random","expected":255,"actual":254},{"offset":4096,"pattern":"Compare","expected":255,"actual":254},{"offset":4096,"pattern":"SolidBits","expected":255,"actual":254},{"offset":4096,"pattern":"Checkerboard","expected":255,"actual":254},{"offset":4096,"pattern":"WalkingOnes","expected":255,"actual":254},{"offset":4096,"pattern":"WalkingZeros","expected":255,"actual":254}],"failure":null}}},
  {"id":6,"subject":{"Volume":"C:\\"},"started_ms":1790000000000,"ended_ms":1790000600000,"outcome":"Problems","data":{"DiskSurface":{"disk":0,"media":"Hdd","disk_bytes":1000000000000,"sector_bytes":512,"from_bytes":0,"to_bytes":1000000000000,"bands":[4,4,4,4,4,4],"unreadable_blocks":2,"regions":["Unread",{"Read":{"worst_ms":12.5}},"Unreadable"],"worst":[{"offset":4096,"ms":480.5}],"unreadable":[8192],"failure":null,"outside_io":false}}},
  {"id":7,"subject":"Pc","started_ms":1790000000000,"ended_ms":1790000600000,"outcome":"Interrupted","data":{"DiskBenchmark":{"volume":"C:\\","bus":"Nvme","media":"Ssd","file_bytes":1000000000,"written_bytes":2000000000,"results":[{"mode":"SeqRead","mb_per_s":1500.5,"iops":90000.5,"max_ms":12.5},{"mode":"SeqWrite","mb_per_s":1500.5,"iops":90000.5,"max_ms":12.5},{"mode":"RandRead32","mb_per_s":1500.5,"iops":90000.5,"max_ms":12.5},{"mode":"RandWrite32","mb_per_s":1500.5,"iops":90000.5,"max_ms":12.5},{"mode":"RandRead1","mb_per_s":1500.5,"iops":90000.5,"max_ms":12.5},{"mode":"RandWrite1","mb_per_s":1500.5,"iops":90000.5,"max_ms":12.5}],"outside_io":false,"failure":null}}},
  {"id":8,"subject":{"Disk":"0 C:"},"started_ms":1790000000000,"ended_ms":1790000600000,"outcome":"Failed","data":{"VolumeCheck":{"volume":"C:\\","exit_code":0,"result":"NoProblems","output":"The file system is fine.","dirty_before":false,"dirty_after":true}}},
  {"id":9,"subject":{"Volume":"C:\\"},"started_ms":1790000000000,"ended_ms":1790000600000,"outcome":"Clean","data":{"VolumeCheck":{"volume":"C:\\","exit_code":null,"result":"NoProblems","output":"The file system is fine.","dirty_before":null,"dirty_after":null}}},
  {"id":10,"subject":"Pc","started_ms":1790000000000,"ended_ms":1790000600000,"outcome":"Problems","data":{"VolumeCheck":{"volume":"C:\\","exit_code":0,"result":"ProblemsFound","output":"The file system is fine.","dirty_before":false,"dirty_after":true}}},
  {"id":11,"subject":{"Disk":"0 C:"},"started_ms":1790000000000,"ended_ms":1790000600000,"outcome":"Interrupted","data":{"VolumeCheck":{"volume":"C:\\","exit_code":null,"result":"ProblemsFound","output":"The file system is fine.","dirty_before":null,"dirty_after":null}}},
  {"id":12,"subject":{"Volume":"C:\\"},"started_ms":1790000000000,"ended_ms":1790000600000,"outcome":"Failed","data":{"VolumeCheck":{"volume":"C:\\","exit_code":0,"result":"CouldNotCheck","output":"The file system is fine.","dirty_before":false,"dirty_after":true}}},
  {"id":13,"subject":"Pc","started_ms":1790000000000,"ended_ms":1790000600000,"outcome":"Clean","data":{"VolumeCheck":{"volume":"C:\\","exit_code":null,"result":"CouldNotCheck","output":"The file system is fine.","dirty_before":null,"dirty_after":null}}},
] satisfies TestRunDto[];

export const testSpanSamples = [
  {"id":5,"kind":"MemoryQuick","subject":{"Disk":"0 C:"},"started_ms":1790000000000,"ended_ms":1790000600000,"outcome":"Clean"},
  {"id":5,"kind":"DiskSurface","subject":{"Disk":"0 C:"},"started_ms":1790000000000,"ended_ms":1790000600000,"outcome":"Problems"},
  {"id":5,"kind":"VolumeCheck","subject":{"Disk":"0 C:"},"started_ms":1790000000000,"ended_ms":1790000600000,"outcome":"Interrupted"},
  {"id":5,"kind":"DiskBenchmark","subject":{"Disk":"0 C:"},"started_ms":1790000000000,"ended_ms":1790000600000,"outcome":"Failed"},
] satisfies TestSpan[];

export const diagnosticResultSamples = [
  "Pass",
  "Cancelled",
  "Incomplete",
  "Errors",
] satisfies DiagnosticResult[];

export const diagnosticDetailSamples = [
  {"memory_mb":16384,"test_type":10,"duration_s":900,"test_count":4,"pages_tested":4000000,"pages_untested":100,"bad_pages":2},
] satisfies DiagnosticDetail[];

export const memoryDiagnosticSamples = [
  {"record_id":800,"ts_ms":1790000000000,"result":"Pass","detail":{"memory_mb":16384,"test_type":10,"duration_s":900,"test_count":4,"pages_tested":4000000,"pages_untested":100,"bad_pages":2}},
  {"record_id":801,"ts_ms":1790000000000,"result":"Cancelled","detail":null},
  {"record_id":802,"ts_ms":1790000000000,"result":"Incomplete","detail":{"memory_mb":16384,"test_type":10,"duration_s":900,"test_count":4,"pages_tested":4000000,"pages_untested":100,"bad_pages":2}},
  {"record_id":803,"ts_ms":1790000000000,"result":"Errors","detail":null},
] satisfies MemoryDiagnostic[];

export const memoryDtoSamples = [
  {"total_bytes":17179869184,"available_bytes":8589934592,"quick_bytes":2000000000,"minutes":[2,4,8]},
  {"total_bytes":17179869184,"available_bytes":8589934592,"quick_bytes":null,"minutes":[2,4,8]},
] satisfies MemoryDto[];

export const diskDtoSamples = [
  {"number":0,"name":"0 C:","media":"Ssd","bus":"Nvme","bytes":1000000000000,"volumes":["C:\\"],"ranges":[{"mode":"SeqRead","low":500.5,"high":3500.5},{"mode":"SeqWrite","low":500.5,"high":3500.5},{"mode":"RandRead32","low":500.5,"high":3500.5},{"mode":"RandWrite32","low":500.5,"high":3500.5},{"mode":"RandRead1","low":500.5,"high":3500.5},{"mode":"RandWrite1","low":500.5,"high":3500.5}]},
  {"number":0,"name":"0 C:","media":"Ssd","bus":"Nvme","bytes":null,"volumes":["C:\\"],"ranges":[{"mode":"SeqRead","low":500.5,"high":3500.5},{"mode":"SeqWrite","low":500.5,"high":3500.5},{"mode":"RandRead32","low":500.5,"high":3500.5},{"mode":"RandWrite32","low":500.5,"high":3500.5},{"mode":"RandRead1","low":500.5,"high":3500.5},{"mode":"RandWrite1","low":500.5,"high":3500.5}]},
] satisfies DiskDto[];

export const volumeDtoSamples = [
  {"root":"C:\\","total_bytes":1000000000000,"free_bytes":400000000000,"system":true,"disk":0,"file_system":"NTFS"},
  {"root":"C:\\","total_bytes":1000000000000,"free_bytes":400000000000,"system":false,"disk":null,"file_system":null},
] satisfies VolumeDto[];

export const surfaceProgressSamples = [
  {"disk":0,"ts_ms":1790000000000,"at_bytes":500000000000,"from_bytes":0,"disk_bytes":1000000000000,"bands":[4,4,4,4,4,4],"unreadable_blocks":1},
] satisfies SurfaceProgress[];

export const scanUnderWaySamples = [
  {"stem":"scan-0","progress":{"disk":0,"ts_ms":1790000000000,"at_bytes":500000000000,"from_bytes":0,"disk_bytes":1000000000000,"bands":[4,4,4,4,4,4],"unreadable_blocks":1}},
] satisfies ScanUnderWay[];

export const runningTestSamples = [
  {"kind":"DiskSurface","subject":{"Disk":"0 C:"},"started_ms":1790000000000,"ends_ms":1790000600000},
  {"kind":"DiskSurface","subject":{"Disk":"0 C:"},"started_ms":1790000000000,"ends_ms":null},
] satisfies RunningTest[];

export const testsDtoSamples = [
  {"memory":{"total_bytes":17179869184,"available_bytes":8589934592,"quick_bytes":2000000000,"minutes":[2,4,8]},"diagnostics":[{"record_id":800,"ts_ms":1790000000000,"result":"Pass","detail":{"memory_mb":16384,"test_type":10,"duration_s":900,"test_count":4,"pages_tested":4000000,"pages_untested":100,"bad_pages":2}},{"record_id":801,"ts_ms":1790000000000,"result":"Cancelled","detail":null},{"record_id":802,"ts_ms":1790000000000,"result":"Incomplete","detail":{"memory_mb":16384,"test_type":10,"duration_s":900,"test_count":4,"pages_tested":4000000,"pages_untested":100,"bad_pages":2}},{"record_id":803,"ts_ms":1790000000000,"result":"Errors","detail":null}],"disks":[{"number":0,"name":"0 C:","media":"Ssd","bus":"Nvme","bytes":1000000000000,"volumes":["C:\\"],"ranges":[{"mode":"SeqRead","low":500.5,"high":3500.5},{"mode":"SeqWrite","low":500.5,"high":3500.5},{"mode":"RandRead32","low":500.5,"high":3500.5},{"mode":"RandWrite32","low":500.5,"high":3500.5},{"mode":"RandRead1","low":500.5,"high":3500.5},{"mode":"RandWrite1","low":500.5,"high":3500.5}]},{"number":0,"name":"0 C:","media":"Ssd","bus":"Nvme","bytes":null,"volumes":["C:\\"],"ranges":[{"mode":"SeqRead","low":500.5,"high":3500.5},{"mode":"SeqWrite","low":500.5,"high":3500.5},{"mode":"RandRead32","low":500.5,"high":3500.5},{"mode":"RandWrite32","low":500.5,"high":3500.5},{"mode":"RandRead1","low":500.5,"high":3500.5},{"mode":"RandWrite1","low":500.5,"high":3500.5}]}],"volumes":[{"root":"C:\\","total_bytes":1000000000000,"free_bytes":400000000000,"system":true,"disk":0,"file_system":"NTFS"},{"root":"C:\\","total_bytes":1000000000000,"free_bytes":400000000000,"system":false,"disk":null,"file_system":null}],"runs":[{"id":1,"subject":"Pc","started_ms":1790000000000,"ended_ms":1790000600000,"outcome":"Clean","data":{"MemoryQuick":{"total_bytes":17179869184,"tested_bytes":2000000000,"rounds":2,"passes":3,"error_count":1,"errors":[{"offset":4096,"pattern":"OwnAddress","expected":255,"actual":254},{"offset":4096,"pattern":"Random","expected":255,"actual":254},{"offset":4096,"pattern":"Compare","expected":255,"actual":254},{"offset":4096,"pattern":"SolidBits","expected":255,"actual":254},{"offset":4096,"pattern":"Checkerboard","expected":255,"actual":254},{"offset":4096,"pattern":"WalkingOnes","expected":255,"actual":254},{"offset":4096,"pattern":"WalkingZeros","expected":255,"actual":254}],"failure":"out of memory"}}},{"id":2,"subject":{"Disk":"0 C:"},"started_ms":1790000000000,"ended_ms":1790000600000,"outcome":"Problems","data":{"DiskSurface":{"disk":0,"media":"Hdd","disk_bytes":1000000000000,"sector_bytes":512,"from_bytes":0,"to_bytes":1000000000000,"bands":[4,4,4,4,4,4],"unreadable_blocks":2,"regions":["Unread",{"Read":{"worst_ms":12.5}},"Unreadable"],"worst":[{"offset":4096,"ms":480.5}],"unreadable":[8192],"failure":"read failed","outside_io":true}}}],"running":{"kind":"DiskSurface","subject":{"Disk":"0 C:"},"started_ms":1790000000000,"ends_ms":1790000600000},"scan":{"stem":"scan-0","progress":{"disk":0,"ts_ms":1790000000000,"at_bytes":500000000000,"from_bytes":0,"disk_bytes":1000000000000,"bands":[4,4,4,4,4,4],"unreadable_blocks":1}},"bench_file_bytes":1000000000,"bench_needed_bytes":2000000000},
  {"memory":{"total_bytes":17179869184,"available_bytes":8589934592,"quick_bytes":null,"minutes":[2,4,8]},"diagnostics":[{"record_id":800,"ts_ms":1790000000000,"result":"Pass","detail":{"memory_mb":16384,"test_type":10,"duration_s":900,"test_count":4,"pages_tested":4000000,"pages_untested":100,"bad_pages":2}},{"record_id":801,"ts_ms":1790000000000,"result":"Cancelled","detail":null},{"record_id":802,"ts_ms":1790000000000,"result":"Incomplete","detail":{"memory_mb":16384,"test_type":10,"duration_s":900,"test_count":4,"pages_tested":4000000,"pages_untested":100,"bad_pages":2}},{"record_id":803,"ts_ms":1790000000000,"result":"Errors","detail":null}],"disks":[{"number":0,"name":"0 C:","media":"Ssd","bus":"Nvme","bytes":1000000000000,"volumes":["C:\\"],"ranges":[{"mode":"SeqRead","low":500.5,"high":3500.5},{"mode":"SeqWrite","low":500.5,"high":3500.5},{"mode":"RandRead32","low":500.5,"high":3500.5},{"mode":"RandWrite32","low":500.5,"high":3500.5},{"mode":"RandRead1","low":500.5,"high":3500.5},{"mode":"RandWrite1","low":500.5,"high":3500.5}]},{"number":0,"name":"0 C:","media":"Ssd","bus":"Nvme","bytes":null,"volumes":["C:\\"],"ranges":[{"mode":"SeqRead","low":500.5,"high":3500.5},{"mode":"SeqWrite","low":500.5,"high":3500.5},{"mode":"RandRead32","low":500.5,"high":3500.5},{"mode":"RandWrite32","low":500.5,"high":3500.5},{"mode":"RandRead1","low":500.5,"high":3500.5},{"mode":"RandWrite1","low":500.5,"high":3500.5}]}],"volumes":[{"root":"C:\\","total_bytes":1000000000000,"free_bytes":400000000000,"system":true,"disk":0,"file_system":"NTFS"},{"root":"C:\\","total_bytes":1000000000000,"free_bytes":400000000000,"system":false,"disk":null,"file_system":null}],"runs":[{"id":1,"subject":"Pc","started_ms":1790000000000,"ended_ms":1790000600000,"outcome":"Clean","data":{"MemoryQuick":{"total_bytes":17179869184,"tested_bytes":2000000000,"rounds":2,"passes":3,"error_count":1,"errors":[{"offset":4096,"pattern":"OwnAddress","expected":255,"actual":254},{"offset":4096,"pattern":"Random","expected":255,"actual":254},{"offset":4096,"pattern":"Compare","expected":255,"actual":254},{"offset":4096,"pattern":"SolidBits","expected":255,"actual":254},{"offset":4096,"pattern":"Checkerboard","expected":255,"actual":254},{"offset":4096,"pattern":"WalkingOnes","expected":255,"actual":254},{"offset":4096,"pattern":"WalkingZeros","expected":255,"actual":254}],"failure":"out of memory"}}},{"id":2,"subject":{"Disk":"0 C:"},"started_ms":1790000000000,"ended_ms":1790000600000,"outcome":"Problems","data":{"DiskSurface":{"disk":0,"media":"Hdd","disk_bytes":1000000000000,"sector_bytes":512,"from_bytes":0,"to_bytes":1000000000000,"bands":[4,4,4,4,4,4],"unreadable_blocks":2,"regions":["Unread",{"Read":{"worst_ms":12.5}},"Unreadable"],"worst":[{"offset":4096,"ms":480.5}],"unreadable":[8192],"failure":"read failed","outside_io":true}}}],"running":null,"scan":null,"bench_file_bytes":1000000000,"bench_needed_bytes":2000000000},
] satisfies TestsDto[];

export const diskDetailDtoSamples = [
  {"health":{"name":"0 C:","read_ms":1790000000000,"predict_failure":true,"attributes":[{"id":5,"current":100,"worst":99,"raw":12}],"nvme":{"critical_warning":0,"temperature_c":41,"available_spare_pct":100,"spare_threshold_pct":10,"percentage_used":3,"media_errors":0}},"nvme":{"health":{"critical_warning":0,"temperature_c":41,"available_spare_pct":100,"spare_threshold_pct":10,"percentage_used":3,"media_errors":0},"data_read_bytes":5000000000,"data_written_bytes":4000000000,"power_cycles":300,"power_on_hours":4000,"unsafe_shutdowns":7,"error_log_entries":0}},
  {"health":null,"nvme":null},
] satisfies DiskDetailDto[];

export const testProgressSamples = [
  {"kind":"memory","pattern":"OwnAddress","rounds":2,"passes":3,"error_count":1},
  {"kind":"memory","pattern":"Random","rounds":2,"passes":3,"error_count":1},
  {"kind":"memory","pattern":"Compare","rounds":2,"passes":3,"error_count":1},
  {"kind":"memory","pattern":"SolidBits","rounds":2,"passes":3,"error_count":1},
  {"kind":"memory","pattern":"Checkerboard","rounds":2,"passes":3,"error_count":1},
  {"kind":"memory","pattern":"WalkingOnes","rounds":2,"passes":3,"error_count":1},
  {"kind":"memory","pattern":"WalkingZeros","rounds":2,"passes":3,"error_count":1},
  {"kind":"benchmark","mode":"SeqRead"},
  {"kind":"benchmark","mode":"SeqWrite"},
  {"kind":"benchmark","mode":"RandRead32"},
  {"kind":"benchmark","mode":"RandWrite32"},
  {"kind":"benchmark","mode":"RandRead1"},
  {"kind":"benchmark","mode":"RandWrite1"},
  {"kind":"surface","disk":0,"ts_ms":1790000000000,"at_bytes":500000000000,"from_bytes":0,"disk_bytes":1000000000000,"bands":[4,4,4,4,4,4],"unreadable_blocks":1},
  {"kind":"memory","pattern":"OwnAddress","rounds":1,"passes":1,"error_count":0},
  {"kind":"benchmark","mode":"SeqRead"},
  {"kind":"surface","disk":0,"ts_ms":1790000000000,"at_bytes":500000000000,"from_bytes":0,"disk_bytes":1000000000000,"bands":[4,4,4,4,4,4],"unreadable_blocks":1},
] satisfies TestProgress[];

export const testFinishedSamples = [
  {"id":8,"run":{"subject":"Pc","started_ms":1790000000000,"ended_ms":1790000600000,"outcome":"Clean","data":{"MemoryQuick":{"total_bytes":17179869184,"tested_bytes":2000000000,"rounds":2,"passes":3,"error_count":1,"errors":[{"offset":4096,"pattern":"OwnAddress","expected":255,"actual":254},{"offset":4096,"pattern":"Random","expected":255,"actual":254},{"offset":4096,"pattern":"Compare","expected":255,"actual":254},{"offset":4096,"pattern":"SolidBits","expected":255,"actual":254},{"offset":4096,"pattern":"Checkerboard","expected":255,"actual":254},{"offset":4096,"pattern":"WalkingOnes","expected":255,"actual":254},{"offset":4096,"pattern":"WalkingZeros","expected":255,"actual":254}],"failure":"out of memory"}}},"exit_code":0},
  {"id":8,"run":{"subject":{"Disk":"0 C:"},"started_ms":1790000000000,"ended_ms":1790000600000,"outcome":"Problems","data":{"DiskSurface":{"disk":0,"media":"Hdd","disk_bytes":1000000000000,"sector_bytes":512,"from_bytes":0,"to_bytes":1000000000000,"bands":[4,4,4,4,4,4],"unreadable_blocks":2,"regions":["Unread",{"Read":{"worst_ms":12.5}},"Unreadable"],"worst":[{"offset":4096,"ms":480.5}],"unreadable":[8192],"failure":"read failed","outside_io":true}}},"exit_code":0},
  {"id":8,"run":{"subject":{"Volume":"C:\\"},"started_ms":1790000000000,"ended_ms":1790000600000,"outcome":"Interrupted","data":{"VolumeCheck":{"volume":"C:\\","exit_code":0,"result":"ProblemsFound","output":"The file system is fine.","dirty_before":false,"dirty_after":true}}},"exit_code":0},
  {"id":null,"run":null,"exit_code":null},
] satisfies TestFinished[];

export const updatePrefsSamples = [
  {"check":true,"last_check_ms":1790000000000,"seen_version":"0.1.0"},
  {"check":null,"last_check_ms":null,"seen_version":null},
] satisfies UpdatePrefs[];

export const notesGroupSamples = [
  {"kind":"Added","items":["One thing."]},
] satisfies NotesGroup[];

export const releaseSamples = [
  {"version":"0.2.0","date":"2026-11-10","groups":[{"kind":"Added","items":["One thing."]}]},
  {"version":"0.2.0","date":null,"groups":[{"kind":"Added","items":["One thing."]}]},
] satisfies Release[];

export const checkResultSamples = [
  {"status":"up_to_date"},
  {"status":"available","version":"0.2.0","date_ms":1790000000000,"groups":[{"kind":"Added","items":["One thing."]}]},
  {"status":"available","version":"0.2.0","date_ms":null,"groups":[]},
] satisfies CheckResult[];

export const progressSamples = [
  {"kind":"chunk","done":10,"total":100},
  {"kind":"installing"},
  {"kind":"failed","message":"download failed"},
  {"kind":"chunk","done":10,"total":null},
] satisfies Progress[];

export const aboutSamples = [
  {"version":"0.1.0","data_bytes":1000000},
  {"version":"0.1.0","data_bytes":null},
] satisfies About[];
