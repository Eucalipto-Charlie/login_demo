use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use sqlx::{sqlite::SqlitePoolOptions, Pool, Sqlite};
use std::net::SocketAddr;
use sqlx::Row;


#[derive(Clone)]
struct AppState {
    pool: Pool<Sqlite>,
}

#[derive(Deserialize)]
struct RegisterRequest {
    username: String,
    password: String,
}

#[derive(Deserialize)]
struct LoginRequest {
    username: String,
    password: String,
}

#[derive(Deserialize)]
struct ChangePasswordRequest {
    username: String,
    old_password: String,
    new_password: String,
    confirm_password: String,
}

#[derive(Serialize)]
struct ApiResponse {
    message: String,
}

/// 初始化数据库（自动创建 users.db）
async fn init_db() -> anyhow::Result<Pool<Sqlite>> {
    let db_url = "sqlite://users.db";
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(db_url)
        .await?;

    // 创建 users 表
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS users (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            username TEXT UNIQUE NOT NULL,
            password TEXT NOT NULL
        );
        "#,
    )
    .execute(&pool)
    .await?;

    Ok(pool)
}

// 注册接口
async fn register(
    State(state): State<AppState>,
    Json(req): Json<RegisterRequest>,
) -> impl IntoResponse {
    let result = sqlx::query("INSERT INTO users (username, password) VALUES (?, ?)")
        .bind(&req.username)
        .bind(&req.password)
        .execute(&state.pool)
        .await;

    match result {
        Ok(_) => (
            StatusCode::OK,
            Json(ApiResponse {
                message: "✅ 用户注册成功".into(),
            }),
        ),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse {
                message: format!("❌ 注册失败: {}", e),
            }),
        ),
    }
}

// 登录接口
async fn login(State(state): State<AppState>, Json(req): Json<LoginRequest>) -> impl IntoResponse {
    let user = sqlx::query("SELECT password FROM users WHERE username = ?")
        .bind(&req.username)
        .fetch_one(&state.pool)
        .await;

    match user {
        Ok(record) => {
            // record.get::<String, _>("password") 获取字段值
            let password: String = record.try_get("password").unwrap();
            if password == req.password {
                (
                    StatusCode::OK,
                    Json(ApiResponse {
                        message: "✅ 登录成功".into(),
                    }),
                )
            } else {
                (
                    StatusCode::UNAUTHORIZED,
                    Json(ApiResponse {
                        message: "❌ 密码错误".into(),
                    }),
                )
            }
        }
        Err(_) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse {
                message: "❌ 用户不存在".into(),
            }),
        ),
    }
}

// 修改密码接口（两次确认新密码）
async fn change_password(
    State(state): State<AppState>,
    Json(req): Json<ChangePasswordRequest>,
) -> impl IntoResponse {
    if req.new_password != req.confirm_password {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse {
                message: "❌ 新密码两次输入不一致".into(),
            }),
        );
    }

    let user = sqlx::query("SELECT password FROM users WHERE username = ?")
        .bind(&req.username)
        .fetch_one(&state.pool)
        .await;

    match user {
        Ok(record) => {
            let password: String = record.try_get("password").unwrap();
            if password != req.old_password {
                return (
                    StatusCode::UNAUTHORIZED,
                    Json(ApiResponse {
                        message: "❌ 原密码错误".into(),
                    }),
                );
            }

            let result = sqlx::query("UPDATE users SET password = ? WHERE username = ?")
                .bind(&req.new_password)
                .bind(&req.username)
                .execute(&state.pool)
                .await;

            match result {
                Ok(_) => (
                    StatusCode::OK,
                    Json(ApiResponse {
                        message: "✅ 密码修改成功".into(),
                    }),
                ),
                Err(e) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse {
                        message: format!("❌ 修改失败: {}", e),
                    }),
                ),
            }
        }
        Err(_) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse {
                message: "❌ 用户不存在".into(),
            }),
        ),
    }
}

// 根路由
async fn root() -> impl IntoResponse {
    Json(ApiResponse {
        message: "Axum + sqlx 登录示例".into(),
    })
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let pool = init_db().await?;
    let state = AppState { pool };

    let app = Router::new()
        .route("/", get(root))
        .route("/register", post(register))
        .route("/login", post(login))
        .route("/change_password", post(change_password))
        .with_state(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("服务已启动: http://{}", addr);
    axum::serve(tokio::net::TcpListener::bind(addr).await?, app).await?;

    Ok(())
}
