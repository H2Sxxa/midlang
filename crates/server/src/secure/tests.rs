use super::*;

#[tokio::test]
async fn creates_and_authenticates_a_persisted_token() {
    let store = AuthStore::connect("sqlite::memory:").await.unwrap();
    let created = store
        .create_token("test-reader", &["reader".to_string()], None)
        .await
        .unwrap();
    let context = store
        .authenticate(&format!("Bearer {}", created.token))
        .await
        .unwrap()
        .unwrap();

    assert_eq!(context.token_id, created.id);
    assert!(context.can("translation:read"));
    assert!(!context.can("translation:write"));
}

#[tokio::test]
async fn revoked_tokens_are_rejected() {
    let store = AuthStore::connect("sqlite::memory:").await.unwrap();
    let created = store
        .create_token("test-writer", &["writer".to_string()], None)
        .await
        .unwrap();
    assert!(store.revoke(&created.id).await.unwrap());
    assert!(
        store
            .authenticate(&format!("Bearer {}", created.token))
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn bootstraps_only_one_admin_token() {
    let store = AuthStore::connect("sqlite::memory:").await.unwrap();
    let first = store.bootstrap_admin_token().await.unwrap().unwrap();
    assert_eq!(first.groups, vec!["admin"]);
    assert!(store.bootstrap_admin_token().await.unwrap().is_none());
}

#[tokio::test]
async fn manages_custom_permission_groups() {
    let store = AuthStore::connect("sqlite::memory:").await.unwrap();
    let catalog = store.list_permissions().await.unwrap();
    assert!(
        catalog
            .iter()
            .any(|permission| permission.name == "permission:manage")
    );

    let create = store
        .create_group(
            "content-reviewer",
            "Reviews content",
            &["translation:read".to_string()],
        )
        .await
        .unwrap();
    let CreateGroupOutcome::Created(group) = create else {
        panic!("expected the group to be created")
    };
    assert_eq!(group.permissions, vec!["translation:read"]);

    assert!(matches!(
        store
            .create_group("content-reviewer", "", &[])
            .await
            .unwrap(),
        CreateGroupOutcome::AlreadyExists
    ));
    assert!(matches!(
        store.create_group("bad group", "", &[]).await.unwrap(),
        CreateGroupOutcome::InvalidInput(_)
    ));
    assert!(matches!(
        store
            .create_group("ghost", "", &["nope".to_string()])
            .await
            .unwrap(),
        CreateGroupOutcome::UnknownPermissions(_)
    ));

    let update = store
        .update_group(
            "content-reviewer",
            "Reviewers",
            &["translation:write".to_string()],
        )
        .await
        .unwrap();
    let UpdateGroupOutcome::Updated(group) = update else {
        panic!("expected the group to be updated")
    };
    assert_eq!(group.permissions, vec!["translation:write"]);

    assert!(matches!(
        store.update_group("reader", "", &[]).await.unwrap(),
        UpdateGroupOutcome::BuiltIn
    ));
    assert!(matches!(
        store.delete_group("reader").await.unwrap(),
        DeleteGroupOutcome::BuiltIn
    ));

    let token = store
        .create_token("reviewer", &["content-reviewer".to_string()], None)
        .await
        .unwrap();
    assert!(matches!(
        store.delete_group("content-reviewer").await.unwrap(),
        DeleteGroupOutcome::InUse(_)
    ));
    assert!(store.revoke(&token.id).await.unwrap());

    store.create_group("temporary", "", &[]).await.unwrap();
    assert!(matches!(
        store.delete_group("temporary").await.unwrap(),
        DeleteGroupOutcome::Deleted
    ));
    assert!(matches!(
        store.delete_group("temporary").await.unwrap(),
        DeleteGroupOutcome::NotFound
    ));
}

#[tokio::test]
async fn implied_permissions_are_granted_automatically() {
    let store = AuthStore::connect("sqlite::memory:").await.unwrap();
    let created = store
        .create_token("test-writer", &["writer".to_string()], None)
        .await
        .unwrap();
    let context = store
        .authenticate(&format!("Bearer {}", created.token))
        .await
        .unwrap()
        .unwrap();

    assert!(context.can("translation:delete"));
    assert!(context.can("translation:write"));
    assert!(context.can("translation:read"));
    assert!(!context.can("permission:manage"));
}

#[tokio::test]
async fn updates_token_groups_and_permissions() {
    let store = AuthStore::connect("sqlite::memory:").await.unwrap();
    let token = store
        .create_token("reader", &["reader".to_string()], None)
        .await
        .unwrap();

    let update = store
        .update_token(&token.id, "reader-v2", &["writer".to_string()], None)
        .await
        .unwrap();
    let UpdateTokenOutcome::Updated(info) = update else {
        panic!("expected the token to be updated")
    };
    assert_eq!(info.name, "reader-v2");
    assert_eq!(info.groups, vec!["writer"]);

    let context = store
        .authenticate(&format!("Bearer {}", token.token))
        .await
        .unwrap()
        .unwrap();
    assert!(context.can("translation:write"));
    assert!(!context.can("token:create"));

    assert!(matches!(
        store
            .update_token(&token.id, "reader", &["ghost".to_string()], None)
            .await
            .unwrap(),
        UpdateTokenOutcome::UnknownGroups(_)
    ));
    assert!(matches!(
        store
            .update_token(&token.id, "", &["reader".to_string()], None)
            .await
            .unwrap(),
        UpdateTokenOutcome::InvalidInput(_)
    ));
}
