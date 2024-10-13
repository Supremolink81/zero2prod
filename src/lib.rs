pub mod configuration;
pub mod routes;
pub mod startup;

use std::net::TcpListener;

use actix_web::{web, App, HttpServer};
use actix_web::dev::Server;
use configuration::{get_configuration, DatabaseSettings};
use sqlx::{PgConnection, Connection, Executor, PgPool};
use uuid::Uuid;

pub struct TestApp {
    pub address: String,
    pub pool: PgPool,
}

pub fn run(listener: TcpListener, db_pool: PgPool) -> std::io::Result<Server> {
    let db_pool = web::Data::new(db_pool);
    let server = HttpServer::new(move || {
            App::new()
                .route("/health_check", web::get().to(routes::health_check))   
                .route("/subscriptions", web::post().to(routes::subscribe)) 
                .app_data(db_pool.clone())
        })
        .listen(listener)?
        .run();

    Ok(server)
}

pub async fn spawn_app() -> TestApp {
    let mut configuration = get_configuration().expect("Failed to read configuration.");
    configuration.database.database_name = Uuid::new_v4().to_string();
    let pool = configure_database(&configuration.database).await;
    let listener = TcpListener::bind("127.0.0.1:0")
        .expect("Failed to bind random port");
    // We retrieve the port assigned to us by the OS
    let port = listener.local_addr().unwrap().port();
    let server = run(listener, pool.clone()).expect("Failed to bind address");
    // We return the application address to the caller!
    let address = format!("http://127.0.0.1:{}", port);
    let _ = tokio::spawn(server);
    TestApp {
        address,
        pool,
    }
}

pub async fn configure_database(config: &DatabaseSettings) -> PgPool {
    // Create database
    let mut connection = PgConnection::connect(&config.connection_string_without_db())
        .await
        .expect("Failed to connect to Postgres");
    connection
        .execute(format!(r#"CREATE DATABASE "{}";"#, config.database_name).as_str())
        .await
        .expect("Failed to create database.");
    // Migrate database
    let connection_pool = PgPool::connect(&config.connection_string())
        .await
        .expect("Failed to connect to Postgres.");
    sqlx::migrate!("./migrations")
        .run(&connection_pool)
        .await
        .expect("Failed to migrate the database");
    connection_pool
}
