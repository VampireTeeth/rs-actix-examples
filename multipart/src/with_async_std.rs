use actix_multipart::Multipart;
use actix_web::web;
use actix_web::{App, error, Error, HttpResponse, HttpServer, Responder};
use futures::{StreamExt, TryStreamExt};
use async_std::prelude::*;

async fn save_file(mut payload: Multipart) -> Result<HttpResponse, Error> {
    while let Ok(Some(mut field)) = TryStreamExt::try_next(&mut payload).await {
        let content_disposition = field
            .content_disposition()
            .ok_or_else(|| error::ParseError::Incomplete)?;

        let filename = content_disposition
            .get_filename()
            .ok_or_else(|| error::ParseError::Incomplete)?;


        let filepath = format!("./tmp/{}", filename);

        let mut f = async_std::fs::File::create(filepath).await?;

        while let Some(chunk) = field.next().await {
            let data = chunk.unwrap();
            f.write_all(&data).await?;
        }

    }
    Ok(HttpResponse::Ok().into())
}

async fn index() -> impl Responder {
    let html = r#"<html>
        <head><title>Upload Test</title></head>
        <body>
            <form target="/" method="post" enctype="multipart/form-data">
                <input type="file" multiple name="file"/>
                <input type="submit" value="Submit"></button>
            </form>
        </body>
    </html>"#;

    HttpResponse::Ok().body(html)
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new().service(
            web::resource("/")
                .route(web::get().to(index))
                .route(web::post().to(save_file)),
        )
    })
    .bind(("0.0.0.0", 9999))?
    .run()
    .await
}
