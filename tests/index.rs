use actix_web::{
    body::MessageBody,
    http::{self, header::ContentType},
    test,
};
use sqlpage::{app_config::AppConfig, webserver::http::main_handler, AppState};

#[actix_web::test]
async fn test_index_ok() {
    let resp = req_path("/").await.unwrap();
    assert_eq!(resp.status(), http::StatusCode::OK);
    let body = test::read_body(resp).await;
    assert!(body.starts_with(b"<!DOCTYPE html>"));
    // the body should contain the strint "It works!" and should not contain the string "error"
    let body = String::from_utf8(body.to_vec()).unwrap();
    assert!(body.contains("It works !"));
    assert!(!body.contains("error"));
}

#[actix_web::test]
async fn test_access_config_forbidden() {
    let resp_result = req_path("/sqlpage/sqlpage.json").await;
    assert!(resp_result.is_err(), "Accessing the config file should be forbidden, but we received a response: {resp_result:?}");
    let resp = resp_result.unwrap_err().error_response();
    assert_eq!(resp.status(), http::StatusCode::FORBIDDEN);
    assert!(
        String::from_utf8_lossy(&resp.into_body().try_into_bytes().unwrap())
            .to_lowercase()
            .contains("forbidden"),
    );
}

#[actix_web::test]
async fn test_404() {
    for f in [
        "/does_not_exist.sql",
        "/does_not_exist.html",
        "/does_not_exist/",
    ] {
        let resp_result = req_path(f).await;
        let resp = resp_result.unwrap_err().error_response();
        assert_eq!(resp.status(), http::StatusCode::NOT_FOUND, "{f} isnt 404");
    }
}

#[actix_web::test]
async fn test_files() {
    // Iterate over all the sql test files in the tests/ directory
    let path = std::path::Path::new("tests/sql_test_files");
    for entry in std::fs::read_dir(path).unwrap() {
        let entry = entry.unwrap();
        let test_file_path = entry.path();
        let test_file_path_string = test_file_path.to_string_lossy().replace('\\', "/");
        let stem = test_file_path.file_stem().unwrap().to_str().unwrap();
        if test_file_path.extension().unwrap_or_default() != "sql" {
            continue;
        }
        let req_str = format!("/{}?x=1", test_file_path_string);
        let resp = req_path(&req_str).await.unwrap();
        let body = test::read_body(resp).await;
        assert!(body.starts_with(b"<!DOCTYPE html>"));
        // the body should contain the strint "It works!" and should not contain the string "error"
        let body = String::from_utf8(body.to_vec()).unwrap();
        let lowercase_body = body.to_lowercase();
        if stem.starts_with("it_works") {
            assert!(
                body.contains("It works !"),
                "{req_str}\n{body}\nexpected to contain: It works !"
            );
            assert!(
                !lowercase_body.contains("error"),
                "{body}\nexpected to not contain: error"
            );
        } else if stem.starts_with("error_") {
            let rest = stem.strip_prefix("error_").unwrap();
            let expected_str = rest.replace('_', " ");
            assert!(
                lowercase_body.contains(&expected_str),
                "{req_str}\n{body}\nexpected to contain: {expected_str}"
            );
            assert!(
                lowercase_body.contains("error"),
                "{req_str}\n{body}\nexpected to contain: error"
            );
        }
    }
}

async fn req_path(path: &str) -> Result<actix_web::dev::ServiceResponse, actix_web::Error> {
    init_log();
    let config = test_config();
    let state = AppState::init(&config).await.unwrap();
    let data = actix_web::web::Data::new(state);
    let req = test::TestRequest::get()
        .uri(path)
        .app_data(data)
        .insert_header(ContentType::plaintext())
        .to_srv_request();
    main_handler(req).await
}

pub fn test_config() -> AppConfig {
    let db_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite::memory:".to_string());
    serde_json::from_str::<AppConfig>(&format!(
        r#"{{
        "database_url": "{}",
        "database_connection_retries": 2,
        "database_connection_acquire_timeout_seconds": 1,
        "allow_exec": true,
        "listen_on": "111.111.111.111:1"
    }}"#,
        db_url
    ))
    .unwrap()
}

fn init_log() {
    let _ = env_logger::builder().is_test(true).try_init();
}
