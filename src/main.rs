use axum::{Router, routing::post, response::IntoResponse, Json, http::StatusCode};
use serde::{Serialize, Deserialize};
use serde_json::json;
use tokio::process::Command;

#[derive(Serialize, Deserialize)]
struct PushData {
    pushed_at: usize,
    pushed: String,
    tag: String,
}

#[derive(Serialize, Deserialize)]
struct Repository {
    comment_count: usize,
    date_created: usize,
    description: String,
    dockerfile: String,
    is_official: bool,
    is_private: bool,
    is_trusted: bool,
    name: String,
    namespace: String,
    owner: String,
    repo_name: String,
    repo_url: String,
    star_count: usize,
    status: String
}

#[derive(Serialize, Deserialize)]
struct ExpectedPayload {
    callback_url: String,
    push_data: Option<PushData>,
    repository: Option<Repository>
}

#[tokio::main]
async fn main() {

    let ip_bind = std::env::var("BIND_URL").expect("Cannot find URL to bind server to");
    println!("{ip_bind}");

    // build our application with a route
    let app = Router::new()
        .route("/", post(post_handler))
        ;
    

    // run our app with hyper
    let _ = axum::Server::bind(&"BIND_URL".parse().unwrap())
        .serve(app.into_make_service())
        .await;
}

use reqwest;
async fn send_success(url: String) {
    let _ = reqwest::Client::new().post(url).header("Content-Type", "application/json").body(json!({
        "state": "success",
        "description": "The docker container has been restarted successfully",
        "context": "Pulled & restarted docker containers"
    }).to_string()).send().await;
}

async fn send_failure(url: String, err: &str) {
    let _ = reqwest::Client::new().post(url).header("Content-Type", "application/json").body(json!({
        "state": "error",
        "description": err,
        "context": "Couldn't restart docker container"
    }).to_string()).send().await;
}

async fn post_handler(payload: Json<ExpectedPayload>) -> impl IntoResponse {
    let mut child = match Command::new("docker")
    .args(&["compose", "down"])
    .current_dir("./")
    .spawn() {
        Ok(ok) => {ok},
        Err(_) => {
            send_failure(payload.callback_url.clone(), "Couldn't create process `docker compose down`").await;
            return (StatusCode::OK, "OK!").into_response()
        },
    };

    match child.wait().await {
        Ok(ok) => {
            if !ok.success() {
                send_failure(payload.callback_url.clone(), "`docker compose down` has failed").await;
                return (StatusCode::OK, "OK!").into_response()
            } 
        },
        Err(err) => {
            send_failure(payload.callback_url.clone(), &format!("`docker compose down` has failed with error: {err}")).await;
            return (StatusCode::OK, "OK!").into_response()
        },
    };

    let mut child = match Command::new("docker")
    .args(&["compose", "pull"])
    .current_dir("./")
    .spawn() {
        Ok(ok) => {ok},
        Err(_) => {
            send_failure(payload.callback_url.clone(), "Couldn't create process `docker compose pull`").await;
            return (StatusCode::OK, "OK!").into_response()
        },
    };

    match child.wait().await {
        Ok(ok) => {
            if !ok.success() {
                send_failure(payload.callback_url.clone(), "`docker compose pull` has failed").await;
                return (StatusCode::OK, "OK!").into_response()
            } 
        },
        Err(err) => {
            send_failure(payload.callback_url.clone(), &format!("`docker compose pull` has failed with error: {err}")).await;
            return (StatusCode::OK, "OK!").into_response()
        },
    };

    
    match Command::new("docker")
    .args(&["compose", "up"])
    .current_dir("./")
    .spawn() {
        Ok(ok) => {ok},
        Err(_) => {
            send_failure(payload.callback_url.clone(), "Couldn't create process `docker compose up`").await;
            return (StatusCode::OK, "OK!").into_response()
        },
    };

    send_success(payload.callback_url.clone()).await;

    (StatusCode::OK, "OK!").into_response()
}

