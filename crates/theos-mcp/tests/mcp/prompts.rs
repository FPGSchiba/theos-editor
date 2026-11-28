//! The guides of the project as MCP prompts (CE-18.1).

use rmcp::model::{ErrorCode, GetPromptRequestParams, Role};
use rmcp::service::ServiceError;

use crate::common::{TestResult, client, fixture_copy};

// AC: CE-18.1
#[tokio::test]
async fn prompts_list_guides() -> TestResult {
    let (_dir, project) = fixture_copy()?;
    let client = client(&project, &[]).await?;
    let mut prompts = client.list_all_prompts().await?;
    prompts.sort_by(|a, b| a.name.cmp(&b.name));
    let seen: Vec<(&str, Option<&str>)> = prompts
        .iter()
        .map(|p| (p.name.as_str(), p.description.as_deref()))
        .collect();
    assert_eq!(
        seen,
        [
            ("atlas", Some("Placing on the atlas")),
            ("quest", Some("quest"))
        ]
    );
    assert!(prompts.iter().all(|p| p.arguments.is_none()));
    client.cancel().await?;
    Ok(())
}

// AC: CE-18.1
#[tokio::test]
async fn prompts_get_returns_text() -> TestResult {
    let (_dir, project) = fixture_copy()?;
    let expected = std::fs::read_to_string(project.join("tools/guides/atlas.md"))?;
    let client = client(&project, &[]).await?;
    let result = client
        .get_prompt(GetPromptRequestParams::new("atlas"))
        .await?;
    assert_eq!(result.messages.len(), 1);
    let message = &result.messages[0];
    assert_eq!(message.role, Role::User);
    let text = &message
        .content
        .as_text()
        .ok_or("the message is not text")?
        .text;
    assert_eq!(text, &expected);
    client.cancel().await?;
    Ok(())
}

// AC: CE-18.1
#[tokio::test]
async fn prompts_unknown_is_invalid_params() -> TestResult {
    let (_dir, project) = fixture_copy()?;
    let client = client(&project, &[]).await?;
    for name in ["nope", "../theos"] {
        let error = client
            .get_prompt(GetPromptRequestParams::new(name))
            .await
            .err()
            .ok_or("an unknown guide must fail")?;
        match error {
            ServiceError::McpError(data) => assert_eq!(data.code, ErrorCode::INVALID_PARAMS),
            other => return Err(format!("not an MCP error: {other:?}").into()),
        }
    }
    client.cancel().await?;
    Ok(())
}

// AC: CE-18.1
#[tokio::test]
async fn new_guide_appears_without_restart() -> TestResult {
    let (_dir, project) = fixture_copy()?;
    let client = client(&project, &[]).await?;
    assert_eq!(client.list_all_prompts().await?.len(), 2);
    std::fs::write(
        project.join("tools/guides/third.md"),
        "# Third guide\n\nText.\n",
    )?;
    let prompts = client.list_all_prompts().await?;
    assert_eq!(prompts.len(), 3);
    assert!(prompts.iter().any(|p| p.name == "third"));
    client.cancel().await?;
    Ok(())
}

// AC: CE-18.1
#[tokio::test]
async fn without_a_project_there_are_no_prompts_and_get_names_the_reason() -> TestResult {
    let empty = tempfile::tempdir()?;
    let folder = empty.path().to_string_lossy().into_owned();
    let client = client(empty.path(), &["--project", &folder]).await?;
    let names: Vec<String> = client
        .list_all_prompts()
        .await?
        .into_iter()
        .map(|p| p.name)
        .collect();
    assert_eq!(names, Vec::<String>::new());
    let error = client
        .get_prompt(GetPromptRequestParams::new("atlas"))
        .await
        .err()
        .ok_or("a missing project must fail")?;
    match error {
        ServiceError::McpError(data) => {
            assert_eq!(data.code, ErrorCode::INVALID_PARAMS);
            assert!(data.message.contains("theos.toml"), "{}", data.message);
        }
        other => return Err(format!("not an MCP error: {other:?}").into()),
    }
    client.cancel().await?;
    Ok(())
}
