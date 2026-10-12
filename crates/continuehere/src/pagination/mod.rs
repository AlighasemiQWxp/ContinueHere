//! Shared numbered-page requests, bounds, and results.

use std::ops::Range;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pagination {
    pub page: usize,
    pub page_size: usize,
}

impl Default for Pagination {
    fn default() -> Self {
        Self {
            page: 1,
            page_size: 50,
        }
    }
}

impl Pagination {
    /// Returns the slice bounds for a one-based page with 1–100 items.
    /// Pages beyond the available items return an empty range.
    pub fn range(&self, total_items: usize) -> Result<Range<usize>, PaginationError> {
        if self.page == 0 {
            return Err(PaginationError::InvalidPage);
        }
        if !(1..=100).contains(&self.page_size) {
            return Err(PaginationError::InvalidPageSize);
        }
        let offset = (self.page - 1)
            .checked_mul(self.page_size)
            .ok_or(PaginationError::Overflow)?;
        let start = offset.min(total_items);
        let end = start + self.page_size.min(total_items - start);
        Ok(start..end)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub page: usize,
    pub page_size: usize,
    pub total_items: usize,
}

impl<T> Page<T> {
    pub fn total_pages(&self) -> usize {
        if self.page_size == 0 {
            return 0;
        }
        self.total_items.div_ceil(self.page_size)
    }

    pub fn has_next_page(&self) -> bool {
        self.page < self.total_pages()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum PaginationError {
    #[error("Page numbers start at 1")]
    InvalidPage,
    #[error("Page size must be between 1 and 100")]
    InvalidPageSize,
    #[error("The requested page offset is too large")]
    Overflow,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ranges_cover_full_partial_empty_and_missing_pages() {
        let request = |page| Pagination {
            page,
            page_size: 50,
        };
        assert_eq!(request(1).range(125), Ok(0..50));
        assert_eq!(request(2).range(125), Ok(50..100));
        assert_eq!(request(3).range(125), Ok(100..125));
        assert_eq!(request(4).range(125), Ok(125..125));
        assert_eq!(request(1).range(0), Ok(0..0));
        assert_eq!(Pagination::default(), request(1));
    }

    #[test]
    fn invalid_requests_and_offset_overflow_are_rejected() {
        assert_eq!(
            Pagination {
                page: 0,
                page_size: 50,
            }
            .range(0),
            Err(PaginationError::InvalidPage)
        );
        for page_size in [0, 101, usize::MAX] {
            assert_eq!(
                Pagination { page: 1, page_size }.range(0),
                Err(PaginationError::InvalidPageSize)
            );
        }
        assert_eq!(
            Pagination {
                page: usize::MAX,
                page_size: 100,
            }
            .range(0),
            Err(PaginationError::Overflow)
        );
        assert_eq!(
            Pagination {
                page: usize::MAX,
                page_size: 1,
            }
            .range(usize::MAX),
            Ok(usize::MAX - 1..usize::MAX)
        );
    }

    #[test]
    fn metadata_handles_empty_exact_partial_and_large_totals() {
        let mut page = Page::<()> {
            items: Vec::new(),
            page: 1,
            page_size: 50,
            total_items: 0,
        };
        assert_eq!(page.total_pages(), 0);
        assert!(!page.has_next_page());
        page.total_items = 100;
        assert_eq!(page.total_pages(), 2);
        assert!(page.has_next_page());
        page.page = 2;
        assert!(!page.has_next_page());
        page.total_items = 101;
        assert_eq!(page.total_pages(), 3);
        assert!(page.has_next_page());
        page.page = 4;
        assert!(!page.has_next_page());
        page.total_items = usize::MAX;
        assert_eq!(page.total_pages(), usize::MAX / 50 + 1);
        page.page_size = 0;
        assert_eq!(page.total_pages(), 0);
        assert!(!page.has_next_page());
    }
}
