//! Code-defined permission catalog and built-in permission groups.

/// A grantable capability. `implies` lists the permissions that are subsets of
/// this one and are therefore granted automatically, so a group only declares
/// the broadest capability it needs: `translation:write` already covers
/// `translation:read`.
pub(super) struct BuiltinPermission {
    pub(super) name: &'static str,
    pub(super) description: &'static str,
    pub(super) implies: &'static [&'static str],
}

/// The permission catalog is owned by the binary: handlers call
/// `require_permission` with these literal names, so runtime code cannot invent
/// capabilities. Entries are ordered from the narrowest capability to the
/// broadest so the UI can present the hierarchy. The table only stores the
/// catalog so groups can reference permission names through a foreign key.
pub(super) const BUILTIN_PERMISSIONS: &[BuiltinPermission] = &[
    BuiltinPermission {
        name: "translation:read",
        description: "Read and list translations, namespaces and statistics",
        implies: &[],
    },
    BuiltinPermission {
        name: "translation:write",
        description: "Create and update translations",
        implies: &["translation:read"],
    },
    BuiltinPermission {
        name: "translation:delete",
        description: "Delete translations",
        implies: &["translation:write"],
    },
    BuiltinPermission {
        name: "translation:import",
        description: "Import translation batches",
        implies: &["translation:write"],
    },
    BuiltinPermission {
        name: "translation:export",
        description: "Export translation artifacts",
        implies: &["translation:read"],
    },
    BuiltinPermission {
        name: "token:read",
        description: "List issued tokens",
        implies: &[],
    },
    BuiltinPermission {
        name: "token:create",
        description: "Issue new tokens",
        implies: &["token:read"],
    },
    BuiltinPermission {
        name: "token:update",
        description: "Change a token name, groups or expiry",
        implies: &["token:read"],
    },
    BuiltinPermission {
        name: "token:rotate",
        description: "Rotate a token secret",
        implies: &["token:read"],
    },
    BuiltinPermission {
        name: "token:revoke",
        description: "Revoke tokens",
        implies: &["token:read"],
    },
    BuiltinPermission {
        name: "permission:read",
        description: "Read the permission catalog and groups",
        implies: &[],
    },
    BuiltinPermission {
        name: "permission:manage",
        description: "Create, update and delete permission groups",
        implies: &["permission:read"],
    },
    BuiltinPermission {
        name: "diagnostic:read",
        description: "Read issues, coverage and changelog diagnostics",
        implies: &[],
    },
    BuiltinPermission {
        name: "audit:read",
        description: "Read audit history",
        implies: &[],
    },
    BuiltinPermission {
        name: "server:admin",
        description: "Administer server-wide settings",
        implies: &[],
    },
];

/// Built-in groups declare only their broadest permissions; the implied
/// subsets are added when a token is authenticated.
pub(super) const BUILTIN_GROUPS: &[(&str, &[&str])] = &[
    ("reader", &["translation:read"]),
    ("writer", &["translation:delete"]),
    (
        "token-manager",
        &[
            "token:create",
            "token:update",
            "token:rotate",
            "token:revoke",
            "permission:read",
        ],
    ),
    ("auditor", &["diagnostic:read", "audit:read"]),
    (
        "admin",
        &[
            "translation:delete",
            "translation:import",
            "translation:export",
            "token:create",
            "token:update",
            "token:rotate",
            "token:revoke",
            "permission:manage",
            "diagnostic:read",
            "audit:read",
            "server:admin",
        ],
    ),
];

/// Longest accepted permission-group name, keeping names safe to embed in URLs.
pub(super) const GROUP_NAME_MAX_LEN: usize = 64;

fn direct_implies(name: &str) -> &'static [&'static str] {
    match BUILTIN_PERMISSIONS
        .iter()
        .find(|permission| permission.name == name)
    {
        Some(permission) => permission.implies,
        None => &[],
    }
}

/// Every permission a grant of `name` also grants, transitively, sorted and
/// de-duplicated.
pub(super) fn transitive_implies(name: &str) -> Vec<String> {
    let mut implied = std::collections::BTreeSet::new();
    let mut pending: Vec<&str> = direct_implies(name).to_vec();
    while let Some(current) = pending.pop() {
        if implied.insert(current.to_string()) {
            pending.extend(direct_implies(current).iter().copied());
        }
    }
    implied.into_iter().collect()
}

/// Expands a set of granted permissions with everything they imply.
pub(super) fn expand_permissions(granted: &[String]) -> Vec<String> {
    let mut effective = std::collections::BTreeSet::new();
    let mut pending: Vec<&str> = granted.iter().map(String::as_str).collect();
    while let Some(current) = pending.pop() {
        if effective.insert(current.to_string()) {
            pending.extend(direct_implies(current).iter().copied());
        }
    }
    effective.into_iter().collect()
}
