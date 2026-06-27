use serde_json::Value;
use std::collections::HashSet;

/// Trait for types that can produce a before/after diff when mutated.
///
/// Implement this manually on any struct you want to audit with snapshots.
/// A derive macro is planned for v0.2.
///
/// # Example
///
/// ```rust
/// use phoxia_auditlog::Auditable;
/// use serde::Serialize;
/// use serde_json::Value;
///
/// #[derive(Clone, Serialize)]
/// struct User {
///     id: String,
///     email: String,
/// }
///
/// impl Auditable for User {
///     fn to_audit_json(&self) -> Value {
///         serde_json::json!({
///             "id": self.id,
///             "email": self.email,
///         })
///     }
/// }
/// ```
pub trait Auditable {
    /// Return a JSON representation of this value for audit snapshots.
    fn to_audit_json(&self) -> Value;

    /// Return the set of field names that differ between `old` and `new`.
    /// Compares top-level JSON keys by value equality.
    fn changed_fields(old: &Self, new: &Self) -> HashSet<String>
    where
        Self: Sized,
    {
        let old_json = old.to_audit_json();
        let new_json = new.to_audit_json();
        let mut changed = HashSet::new();

        if let (Value::Object(old_map), Value::Object(new_map)) = (&old_json, &new_json) {
            for (key, new_val) in new_map {
                match old_map.get(key) {
                    Some(old_val) if old_val != new_val => {
                        changed.insert(key.clone());
                    }
                    None => {
                        changed.insert(key.clone());
                    }
                    _ => {}
                }
            }
            // Also catch removed keys
            for key in old_map.keys() {
                if !new_map.contains_key(key) {
                    changed.insert(key.clone());
                }
            }
        }

        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone)]
    struct TestUser {
        id: String,
        name: String,
        email: String,
    }

    impl Auditable for TestUser {
        fn to_audit_json(&self) -> Value {
            serde_json::json!({
                "id": self.id,
                "name": self.name,
                "email": self.email,
            })
        }
    }

    #[test]
    fn detects_changed_fields() {
        let old = TestUser {
            id: "1".into(),
            name: "Alice".into(),
            email: "alice@example.com".into(),
        };
        let new = TestUser {
            id: "1".into(),
            name: "Bob".into(),
            email: "alice@example.com".into(),
        };

        let changed = TestUser::changed_fields(&old, &new);
        assert!(changed.contains("name"));
        assert!(!changed.contains("id"));
        assert!(!changed.contains("email"));
    }

    #[test]
    fn detects_no_changes() {
        let old = TestUser {
            id: "1".into(),
            name: "Alice".into(),
            email: "alice@example.com".into(),
        };
        let new = old.clone();

        let changed = TestUser::changed_fields(&old, &new);
        assert!(changed.is_empty());
    }

    #[test]
    fn detects_removed_field_in_new_version() {
        // Simulate: old struct has 3 fields, new struct only includes 2 in JSON
        let old = TestUser {
            id: "1".into(),
            name: "Alice".into(),
            email: "alice@example.com".into(),
        };
        // Same struct but new version serializes without email
        let new = TestUser {
            id: "1".into(),
            name: "Alice".into(),
            email: String::new(), // empty = "removed" from JSON perspective
        };

        let changed = TestUser::changed_fields(&old, &new);
        assert!(changed.contains("email"), "changed-to-empty should be detected");
    }
}
