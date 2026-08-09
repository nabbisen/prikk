//! Test-only failure seams for required filesystem boundaries.

#[cfg(test)]
use prikk_error::PrikkError;
use prikk_error::Result;
#[cfg(test)]
use std::cell::RefCell;
#[cfg(test)]
use std::sync::{Arc, Barrier};

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Point {
    CreatedDirectoryParentSync,
    ObservedDirectoryParentSync,
    DirectoryCreate,
    MutableFileSync,
    MutableRename,
    MutableParentSync,
    PromotionDestinationSync,
    PromotionRename,
    PromotionSourceSync,
    RequiredDirectorySync,
    RequiredFileSync,
    RequiredOpen,
    AppendWrite,
    Truncate,
    Unlink,
    CleanupDirectorySync,
    ImmutableCleanupSync,
    ImmutableFileSync,
    ImmutableInstall,
    ImmutableInstallUnsupported,
    ImmutableInstallNoSys,
    ImmutableInstallPermission,
    ImmutableInstallSync,
    ImmutableTempUnlink,
}

#[cfg(test)]
std::thread_local! {
    static NEXT: RefCell<Option<(Point, usize)>> = const { RefCell::new(None) };
    static DIRECTORY_CREATE_BARRIER: RefCell<Option<Arc<Barrier>>> = const { RefCell::new(None) };
    static IMMUTABLE_INSTALL_BARRIER: RefCell<Option<Arc<Barrier>>> = const { RefCell::new(None) };
}

#[cfg(test)]
pub(crate) fn fail_once(point: Point) {
    fail_after(point, 0);
}

#[cfg(test)]
pub(crate) fn fail_after(point: Point, matching_calls_to_skip: usize) {
    NEXT.with(|next| *next.borrow_mut() = Some((point, matching_calls_to_skip)));
}

#[cfg(test)]
pub(crate) fn set_directory_create_barrier(barrier: Arc<Barrier>) {
    DIRECTORY_CREATE_BARRIER.with(|slot| *slot.borrow_mut() = Some(barrier));
}

#[cfg(test)]
pub(crate) fn set_immutable_install_barrier(barrier: Arc<Barrier>) {
    IMMUTABLE_INSTALL_BARRIER.with(|slot| *slot.borrow_mut() = Some(barrier));
}

pub(super) fn wait_at_directory_create() {
    wait_at_test_barrier(TestBarrier::DirectoryCreate);
}

pub(super) fn wait_at_immutable_install() {
    wait_at_test_barrier(TestBarrier::ImmutableInstall);
}

pub(super) fn created_directory_parent_sync() -> Result<()> {
    check_test_point(TestPoint::CreatedDirectoryParentSync)
}

pub(super) fn observed_directory_parent_sync() -> Result<()> {
    check_test_point(TestPoint::ObservedDirectoryParentSync)
}

pub(super) fn directory_create() -> Result<()> {
    check_test_point(TestPoint::DirectoryCreate)
}

pub(super) fn mutable_file_sync() -> Result<()> {
    check_test_point(TestPoint::MutableFileSync)
}

pub(super) fn mutable_rename() -> Result<()> {
    check_test_point(TestPoint::MutableRename)
}

pub(super) fn mutable_parent_sync() -> Result<()> {
    check_test_point(TestPoint::MutableParentSync)
}

pub(super) fn promotion_destination_sync() -> Result<()> {
    check_test_point(TestPoint::PromotionDestinationSync)
}

pub(super) fn promotion_rename() -> Result<()> {
    check_test_point(TestPoint::PromotionRename)
}

pub(super) fn promotion_source_sync() -> Result<()> {
    check_test_point(TestPoint::PromotionSourceSync)
}

pub(super) fn required_directory_sync() -> Result<()> {
    check_test_point(TestPoint::RequiredDirectorySync)
}

pub(super) fn required_file_sync() -> Result<()> {
    check_test_point(TestPoint::RequiredFileSync)
}

pub(super) fn required_open() -> Result<()> {
    check_test_point(TestPoint::RequiredOpen)
}

pub(super) fn append_write() -> Result<()> {
    check_test_point(TestPoint::AppendWrite)
}

pub(super) fn truncate() -> Result<()> {
    check_test_point(TestPoint::Truncate)
}

pub(super) fn unlink() -> Result<()> {
    check_test_point(TestPoint::Unlink)
}

pub(super) fn cleanup_directory_sync() -> Result<()> {
    check_test_point(TestPoint::CleanupDirectorySync)
}

pub(super) fn immutable_file_sync() -> Result<()> {
    check_test_point(TestPoint::ImmutableFileSync)
}

pub(super) fn immutable_install() -> Result<()> {
    check_test_point(TestPoint::ImmutableInstall)
}

/// Injects the errno `LinuxDurability`/`MacosDurability`'s `linkat`-based no-clobber install could
/// plausibly surface if the destination filesystem or OS policy refuses a hardlink install. The three
/// `rustix::io::Errno` values named are portable POSIX errno wrappers (present in `rustix`'s `Errno`
/// type on every `libc`-backed target, `apple` included) — the injection mechanism and the values it
/// injects are shared between platforms. What is **not** re-verified here: whether these are the
/// *actual* errnos APFS's `linkat` returns for "not supported" (DC-81 §1 Q4/addendum-1: a genuine port,
/// not a recompile — needs macOS CI confirmation, not asserted from this file alone).
#[cfg(any(target_os = "linux", target_os = "macos"))]
pub(super) fn immutable_install_error() -> Option<rustix::io::Errno> {
    for (point, error) in [
        (
            TestPoint::ImmutableInstallUnsupported,
            rustix::io::Errno::OPNOTSUPP,
        ),
        (TestPoint::ImmutableInstallNoSys, rustix::io::Errno::NOSYS),
        (
            TestPoint::ImmutableInstallPermission,
            rustix::io::Errno::PERM,
        ),
    ] {
        if check_test_point(point).is_err() {
            return Some(error);
        }
    }
    None
}

pub(super) fn immutable_install_sync() -> Result<()> {
    check_test_point(TestPoint::ImmutableInstallSync)
}

pub(super) fn immutable_temp_unlink() -> Result<()> {
    check_test_point(TestPoint::ImmutableTempUnlink)
}

pub(super) fn immutable_cleanup_sync() -> Result<()> {
    check_test_point(TestPoint::ImmutableCleanupSync)
}

#[derive(Clone, Copy)]
enum TestPoint {
    CreatedDirectoryParentSync,
    ObservedDirectoryParentSync,
    DirectoryCreate,
    MutableFileSync,
    MutableRename,
    MutableParentSync,
    PromotionDestinationSync,
    PromotionRename,
    PromotionSourceSync,
    RequiredDirectorySync,
    RequiredFileSync,
    RequiredOpen,
    AppendWrite,
    Truncate,
    Unlink,
    CleanupDirectorySync,
    ImmutableCleanupSync,
    ImmutableFileSync,
    ImmutableInstall,
    ImmutableInstallUnsupported,
    ImmutableInstallNoSys,
    ImmutableInstallPermission,
    ImmutableInstallSync,
    ImmutableTempUnlink,
}

fn check_test_point(point: TestPoint) -> Result<()> {
    #[cfg(test)]
    {
        check(point.into())
    }
    #[cfg(not(test))]
    {
        let _ = point;
        Ok(())
    }
}

#[cfg(test)]
impl From<TestPoint> for Point {
    fn from(value: TestPoint) -> Self {
        match value {
            TestPoint::CreatedDirectoryParentSync => Self::CreatedDirectoryParentSync,
            TestPoint::ObservedDirectoryParentSync => Self::ObservedDirectoryParentSync,
            TestPoint::DirectoryCreate => Self::DirectoryCreate,
            TestPoint::MutableFileSync => Self::MutableFileSync,
            TestPoint::MutableRename => Self::MutableRename,
            TestPoint::MutableParentSync => Self::MutableParentSync,
            TestPoint::PromotionDestinationSync => Self::PromotionDestinationSync,
            TestPoint::PromotionRename => Self::PromotionRename,
            TestPoint::PromotionSourceSync => Self::PromotionSourceSync,
            TestPoint::RequiredDirectorySync => Self::RequiredDirectorySync,
            TestPoint::RequiredFileSync => Self::RequiredFileSync,
            TestPoint::RequiredOpen => Self::RequiredOpen,
            TestPoint::AppendWrite => Self::AppendWrite,
            TestPoint::Truncate => Self::Truncate,
            TestPoint::Unlink => Self::Unlink,
            TestPoint::CleanupDirectorySync => Self::CleanupDirectorySync,
            TestPoint::ImmutableCleanupSync => Self::ImmutableCleanupSync,
            TestPoint::ImmutableFileSync => Self::ImmutableFileSync,
            TestPoint::ImmutableInstall => Self::ImmutableInstall,
            TestPoint::ImmutableInstallUnsupported => Self::ImmutableInstallUnsupported,
            TestPoint::ImmutableInstallNoSys => Self::ImmutableInstallNoSys,
            TestPoint::ImmutableInstallPermission => Self::ImmutableInstallPermission,
            TestPoint::ImmutableInstallSync => Self::ImmutableInstallSync,
            TestPoint::ImmutableTempUnlink => Self::ImmutableTempUnlink,
        }
    }
}

#[derive(Clone, Copy)]
enum TestBarrier {
    DirectoryCreate,
    ImmutableInstall,
}

fn wait_at_test_barrier(barrier: TestBarrier) {
    #[cfg(test)]
    {
        let slot = match barrier {
            TestBarrier::DirectoryCreate => &DIRECTORY_CREATE_BARRIER,
            TestBarrier::ImmutableInstall => &IMMUTABLE_INSTALL_BARRIER,
        };
        slot.with(|slot| {
            if let Some(barrier) = slot.borrow_mut().take() {
                barrier.wait();
            }
        });
    }
    #[cfg(not(test))]
    {
        let _ = barrier;
    }
}

#[cfg(test)]
fn check(point: Point) -> Result<()> {
    NEXT.with(|next| {
        let mut next = next.borrow_mut();
        match next.as_mut() {
            Some((selected, remaining)) if *selected == point => {
                if *remaining == 0 {
                    *next = None;
                    return Err(PrikkError::Io(format!(
                        "injected filesystem failure at {point:?}"
                    )));
                }
                *remaining = remaining.saturating_sub(1);
            }
            _ => {}
        }
        Ok(())
    })
}
