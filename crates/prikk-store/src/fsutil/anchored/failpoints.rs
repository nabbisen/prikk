//! Test-only failure seams for required filesystem boundaries.

#[cfg(test)]
use prikk_error::PrikkError;
use prikk_error::Result;
#[cfg(test)]
use std::cell::RefCell;

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
}

#[cfg(test)]
std::thread_local! {
    static NEXT: RefCell<Option<(Point, usize)>> = const { RefCell::new(None) };
}

#[cfg(test)]
pub(crate) fn fail_once(point: Point) {
    fail_after(point, 0);
}

#[cfg(test)]
pub(crate) fn fail_after(point: Point, matching_calls_to_skip: usize) {
    NEXT.with(|next| *next.borrow_mut() = Some((point, matching_calls_to_skip)));
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
        }
    }
}

#[cfg(test)]
fn check(point: Point) -> Result<()> {
    NEXT.with(|next| {
        let mut next = next.borrow_mut();
        if let Some((selected, remaining)) = next.as_mut()
            && *selected == point
        {
            if *remaining == 0 {
                *next = None;
                return Err(PrikkError::Io(format!(
                    "injected filesystem failure at {point:?}"
                )));
            }
            *remaining = remaining.saturating_sub(1);
        }
        Ok(())
    })
}
