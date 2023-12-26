use lazy_static::lazy_static;
use raw_cpuid::{CpuId, CpuIdReaderNative};

lazy_static! {
    pub static ref CPUID: CpuId<CpuIdReaderNative> = CpuId::new();
}
