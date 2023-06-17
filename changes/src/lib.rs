#[derive(Debug, PartialEq, Eq)]
pub enum Change<T> {
    Added(T),
    Removed(T),
}

use Change::{Added, Removed};

pub fn changes<'t, T: Eq>(before: &'t [T], after: &'t [T]) -> Vec<Change<&'t T>> {
    after
        .iter()
        .filter_map(|x| {
            if before.contains(x) {
                None
            } else {
                Some(Added(x))
            }
        })
        .chain(before.iter().filter_map(|x| {
            if after.contains(x) {
                None
            } else {
                Some(Removed(x))
            }
        }))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_unchanged() {
        let changes = changes::<()>(&[], &[]);
        assert!(changes.is_empty());
    }

    #[test]
    fn empty_add_one() {
        let changes = changes(&[], &[42]);
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0], Added(&42));
    }

    #[test]
    fn empty_add_multiple() {
        let changes = changes(&[], &[1, 2, 3, 4, 5]);
        assert_eq!(changes.len(), 5);
        assert!(changes.contains(&Added(&1)));
        assert!(changes.contains(&Added(&2)));
        assert!(changes.contains(&Added(&3)));
        assert!(changes.contains(&Added(&4)));
        assert!(changes.contains(&Added(&5)));
    }

    #[test]
    fn non_empty_add_multiple() {
        let changes = changes(&[4, 5, 6, 7], &[1, 2, 3, 4, 5, 6, 7]);
        assert_eq!(changes.len(), 3);
        assert!(changes.contains(&Added(&1)));
        assert!(changes.contains(&Added(&2)));
        assert!(changes.contains(&Added(&3)));
    }

    #[test]
    fn remove_one() {
        let changes = changes(&[1, 2, 3], &[1, 3]);
        assert_eq!(changes.len(), 1);
        assert!(changes.contains(&Removed(&2)));
    }

    #[test]
    fn remove_multiple() {
        let changes = changes(&[1, 2, 3, 4, 5], &[1, 4]);
        assert_eq!(changes.len(), 3);
        assert!(changes.contains(&Removed(&2)));
        assert!(changes.contains(&Removed(&3)));
        assert!(changes.contains(&Removed(&5)));
    }

    #[test]
    fn add_and_remove() {
        let changes = changes(&[4, 5, 6, 7], &[1, 2, 3, 4, 5]);
        assert_eq!(changes.len(), 5);
        assert!(changes.contains(&Added(&1)));
        assert!(changes.contains(&Added(&2)));
        assert!(changes.contains(&Added(&3)));
        assert!(changes.contains(&Removed(&6)));
        assert!(changes.contains(&Removed(&7)));
    }
}
