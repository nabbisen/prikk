//! Test-only failure seams for required filesystem boundaries.

#[cfg(test)]
use prikk_error::PrikkError;
use prikk_error::Result;
#[cfg(test)]
use std::cell::RefCell;
#[cfg(test)]
use std::sync::{Arc, Barrier};

// DC-98 Stage 2, classification rows #10-#14: these five variants inject at directory-entry
// syncing, which has no Windows counterpart at all -- `windows.rs` performs no directory sync
// anywhere (the same reason `durable_directory_entry` is a documented no-op there). Gated to
// Linux/macOS specifically rather than left reachable-but-uncalled on Windows, so the absence is a
// compile-time fact matching the classification, not a silent dead-code warning `-D warnings` would
// otherwise catch on every Windows build once this module compiles there too.
#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Point {
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    CreatedDirectoryParentSync,
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    ObservedDirectoryParentSync,
    DirectoryCreate,
    MutableFileSync,
    MutableRename,
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    MutableParentSync,
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    RequiredDirectorySync,
    RequiredFileSync,
    RequiredOpen,
    AppendWrite,
    Truncate,
    Unlink,
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    CleanupDirectorySync,
}

#[cfg(test)]
std::thread_local! {
    static NEXT: RefCell<Option<(Point, usize)>> = const { RefCell::new(None) };
    static DIRECTORY_CREATE_BARRIER: RefCell<Option<Arc<Barrier>>> = const { RefCell::new(None) };
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

pub(super) fn wait_at_directory_create() {
    wait_at_test_barrier(TestBarrier::DirectoryCreate);
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
pub(super) fn created_directory_parent_sync() -> Result<()> {
    check_test_point(TestPoint::CreatedDirectoryParentSync)
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
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

#[cfg(any(target_os = "linux", target_os = "macos"))]
pub(super) fn mutable_parent_sync() -> Result<()> {
    check_test_point(TestPoint::MutableParentSync)
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
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

#[cfg(any(target_os = "linux", target_os = "macos"))]
pub(super) fn cleanup_directory_sync() -> Result<()> {
    check_test_point(TestPoint::CleanupDirectorySync)
}

#[derive(Clone, Copy)]
enum TestPoint {
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    CreatedDirectoryParentSync,
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    ObservedDirectoryParentSync,
    DirectoryCreate,
    MutableFileSync,
    MutableRename,
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    MutableParentSync,
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    RequiredDirectorySync,
    RequiredFileSync,
    RequiredOpen,
    AppendWrite,
    Truncate,
    Unlink,
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    CleanupDirectorySync,
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
            #[cfg(any(target_os = "linux", target_os = "macos"))]
            TestPoint::CreatedDirectoryParentSync => Self::CreatedDirectoryParentSync,
            #[cfg(any(target_os = "linux", target_os = "macos"))]
            TestPoint::ObservedDirectoryParentSync => Self::ObservedDirectoryParentSync,
            TestPoint::DirectoryCreate => Self::DirectoryCreate,
            TestPoint::MutableFileSync => Self::MutableFileSync,
            TestPoint::MutableRename => Self::MutableRename,
            #[cfg(any(target_os = "linux", target_os = "macos"))]
            TestPoint::MutableParentSync => Self::MutableParentSync,
            #[cfg(any(target_os = "linux", target_os = "macos"))]
            TestPoint::RequiredDirectorySync => Self::RequiredDirectorySync,
            TestPoint::RequiredFileSync => Self::RequiredFileSync,
            TestPoint::RequiredOpen => Self::RequiredOpen,
            TestPoint::AppendWrite => Self::AppendWrite,
            TestPoint::Truncate => Self::Truncate,
            TestPoint::Unlink => Self::Unlink,
            #[cfg(any(target_os = "linux", target_os = "macos"))]
            TestPoint::CleanupDirectorySync => Self::CleanupDirectorySync,
        }
    }
}

#[derive(Clone, Copy)]
enum TestBarrier {
    DirectoryCreate,
}

fn wait_at_test_barrier(barrier: TestBarrier) {
    #[cfg(test)]
    {
        let slot = match barrier {
            TestBarrier::DirectoryCreate => &DIRECTORY_CREATE_BARRIER,
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
