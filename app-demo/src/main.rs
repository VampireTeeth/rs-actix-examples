use actix_web::{
    App,
    HttpServer,
    Responder,
    HttpRequest,
    HttpResponse,
    error,
    body,
    ResponseError,
    FromRequest,
};
use actix_web::web;
use actix_web::http::{
    StatusCode,
};

use actix_web::guard;

use serde::{Deserialize, Serialize};
use serde_json;

use std::sync::{Arc, Mutex};

use std::fmt::{Display, Formatter, Result};

struct MyCounter {
    count: Mutex<i32>,
}


#[derive(Debug, Deserialize, Serialize)]
struct User {
    name: String,
    age: i32,
}

#[derive(Debug, Serialize)]
struct UserOpError {
    error_code: i32,
    msg: String,
}

impl UserOpError {
    fn new(error_code: i32, msg: &str) -> Self {
        Self {
            error_code,
            msg: String::from(msg),
        }
    }
}

impl Display for UserOpError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "{}", self.msg)
    }
}

impl ResponseError for UserOpError {

    fn status_code(&self) -> StatusCode {
        StatusCode::INTERNAL_SERVER_ERROR
    }

    fn error_response(&self) -> HttpResponse {
        HttpResponse::new(self.status_code())
            .set_body(body::Body::from(serde_json::to_string(&self).unwrap()))
    }

}


async fn index(req: HttpRequest, data: web::Data<MyCounter>) -> impl Responder {
    let count1 = {
        let mut m = data.count.lock().unwrap();
        *m += 1;
        println!("Data is {}", *m);
        *m
    };

    let count2 = {
        let mut m = req.app_data::<Arc<MyCounter>>().unwrap().count.lock().unwrap();
        println!("App Data is {}", *m);
        *m += 1;
        *m
    };

    HttpResponse::Ok()
        .body(format!("Data is {}, App Data is {}", count1, count2))
}

async fn search_users(path: web::Path<User>) -> impl Responder {
    HttpResponse::Ok()
        .content_type("text/plain; charset=utf-8")
        .body(format!("Your user search: {:?}", *path))
}

async fn create_user(query: web::Query<User>) -> impl Responder {
    HttpResponse::Ok()
        .content_type("text/plain; charset=utf-8")
        .body(format!("Your user is created: {:?}", *query))
}

fn user_extract_error_handler(config: web::QueryConfig) -> web::QueryConfig {
    config.error_handler(|err, _req| {
        let msg = format!("{}", err);
        error::InternalError::from_response(
            err, UserOpError::new(123, &msg).error_response())
            .into()
    })
}

fn users_config(config: &mut web::ServiceConfig) {
    config.service(
        web::scope("/users")
            .app_data(
                web::Query::<User>::configure(user_extract_error_handler)
            )
            .route("/search/{name}/{age}", web::get().to(search_users))
            .route("/create", web::route().guard(guard::Post()).to(create_user))
    );
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    let my_counter = MyCounter {count: Mutex::new(0)};
    let my_counter_arc = Arc::new(my_counter);


    HttpServer::new(move || {
        App::new()
            .data(MyCounter { count: Mutex::new(100)})
            .app_data(my_counter_arc.clone())
            .configure(users_config)
            .service(
                web::resource("/index")
                    .route(web::get().to(index)))
    })
    .bind(("0.0.0.0", 8888))?
    .run()
    .await
}
