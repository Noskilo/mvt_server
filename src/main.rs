mod error;

use std::{collections::HashMap, sync::RwLock};

use crate::error::ParsableRequestParam;
use actix_web::{
    get,
    web::{self, Data},
    App, HttpRequest, HttpServer, Responder, middleware,
};
use error::{TransectError, TransectErrorCode};
use sqlx::{postgres::PgPoolOptions, PgPool, Row};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Create a connection pool
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&dotenv::var("DATABASE_URL").unwrap())
        .await
        .expect("Failed to create pool");

    let mvt_data = Data::new(RwLock::new(HashMap::<String, Vec<u8>>::new()));

    HttpServer::new(move || {
        App::new()
            .wrap(middleware::Compress::default())
            .app_data(Data::new(pool.clone()))
            .app_data(Data::clone(&mvt_data))
            .service(index)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}

#[get("/{z}/{x}/{y}.{format}")]
async fn index(req: HttpRequest, pool: web::Data<PgPool>) -> Result<impl Responder, TransectError> {
    let mvt_data = req
        .app_data::<Data<RwLock<HashMap<String, Vec<u8>>>>>()
        .unwrap();

    let format: String = req.match_info().get("format").parsable("format")?;
    let z: u32 = req.match_info().get("z").parsable("z")?;
    let x: u32 = req.match_info().get("x").parsable("x")?;
    let y: u32 = req.match_info().get("y").parsable("y")?;

    let key = format!("/{z}/{x}/{y}.{format}");

    if format != "mvt" {
        return Err(TransectError {
            title: None,
            detail: Some(format!("The format '{format}' is not supported.")),
            code: Some(TransectErrorCode::InvalidInput),
        });
    }

    let read_cached_mvt = mvt_data.read().unwrap();

    if read_cached_mvt.contains_key(key.as_str()) {
        let cached_mvt = read_cached_mvt.get(key.as_str());

        match cached_mvt {
            Some(mvt) => {
                return Ok(mvt.clone());
            }
            None => {}
        }
    }

    std::mem::drop(read_cached_mvt);

    let query = format!(
        r#"
    WITH 
    bounds AS ( 
        SELECT ST_Transform(ST_TileEnvelope({z}, {x}, {y}), 3857) AS geom
    ), 
    mvtgeom AS ( 
        SELECT ST_AsMVTGeom(ST_Transform(p.geometry, 3857),  bounds.geom) AS geom, _id
        FROM projects p, bounds 
        WHERE ST_Transform(p.geometry, 3857) && bounds.geom
        AND p.deleted_at is null
    ) 
    SELECT ST_AsMVT(mvtgeom.*) FROM mvtgeom
            "#
    );

    let row = sqlx::query(&query)
        .fetch_one(pool.get_ref())
        .await
        .map_err(|_| TransectError {
            title: None,
            detail: Some("An unexpected error occurred.".to_string()),
            code: Some(TransectErrorCode::DBError),
        })?;

    let mvt = row.get::<Vec<u8>, _>("st_asmvt");

    let mut write_cached_mvt = mvt_data.write().unwrap();
    write_cached_mvt.insert(key, mvt.clone());

    Ok(mvt)
}
