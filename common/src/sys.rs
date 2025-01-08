use lazy_static::lazy_static;
use sysinfo::{
    CpuRefreshKind, MINIMUM_CPU_UPDATE_INTERVAL, MemoryRefreshKind, RefreshKind, System,
};
use tokio::sync::Mutex;

lazy_static! {
    pub static ref SYSTEM: Mutex<System> = {
        let mut sys = System::new();
        sys.refresh_specifics(RefreshKind::new().with_cpu(CpuRefreshKind::everything()));
        sys.refresh_specifics(
            RefreshKind::new().with_memory(MemoryRefreshKind::new().with_ram().with_swap()),
        );
        std::thread::sleep(MINIMUM_CPU_UPDATE_INTERVAL);
        sys.refresh_cpu_usage();
        Mutex::new(sys)
    };
}
