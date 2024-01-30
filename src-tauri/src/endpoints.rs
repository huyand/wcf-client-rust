use serde::Serialize;
use std::convert::Infallible;
use std::sync::{Arc, Mutex};
use utoipa::{OpenApi, ToSchema};
use utoipa_swagger_ui::Config;
use warp::reply::Json;
use warp::{
    http::Uri,
    hyper::{Response, StatusCode},
    path::{FullPath, Tail},
    Filter, Rejection, Reply,
};

use crate::wcferry::WeChat;

#[derive(Serialize, ToSchema, Clone)]
#[aliases(ApiResponseBool = ApiResponse<bool>, ApiResponseString = ApiResponse<String>)]
struct ApiResponse<T>
where
    T: Serialize,
{
    status: u16,
    error: Option<String>,
    data: Option<T>,
}

pub fn get_routes(
    wechat: Arc<Mutex<WeChat>>,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    let config = Arc::new(Config::from("/api-doc.json"));

    #[derive(OpenApi)]
    #[openapi(
        paths(is_login, get_self_wxid),
        components(schemas(ApiResponse<bool>, ApiResponse<String>)),
        tags((name = "WCF", description = "玩微信的接口"))
    )]
    struct ApiDoc;

    let api_doc = warp::path("api-doc.json")
        .and(warp::get())
        .map(|| warp::reply::json(&ApiDoc::openapi()));

    let swagger_ui = warp::path("swagger")
        .and(warp::get())
        .and(warp::path::full())
        .and(warp::path::tail())
        .and(warp::any().map(move || config.clone()))
        .and_then(serve_swagger);

    fn islogin(
        wechat: Arc<Mutex<WeChat>>,
    ) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
        warp::path("islogin")
            .and(warp::any().map(move || wechat.clone()))
            .and_then(is_login)
    }

    fn selfwxid(
        wechat: Arc<Mutex<WeChat>>,
    ) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
        warp::path("selfwxid")
            .and(warp::any().map(move || wechat.clone()))
            .and_then(get_self_wxid)
    }

    api_doc
        .or(swagger_ui)
        .or(islogin(wechat.clone()).or(selfwxid(wechat.clone())))
}

async fn serve_swagger(
    full_path: FullPath,
    tail: Tail,
    config: Arc<Config<'static>>,
) -> Result<Box<dyn Reply + 'static>, Rejection> {
    if full_path.as_str() == "/swagger" {
        return Ok(Box::new(warp::redirect::found(Uri::from_static(
            "/swagger/",
        ))));
    }

    let path = tail.as_str();
    match utoipa_swagger_ui::serve(path, config) {
        Ok(file) => {
            if let Some(file) = file {
                Ok(Box::new(
                    Response::builder()
                        .header("Content-Type", file.content_type)
                        .body(file.bytes),
                ))
            } else {
                Ok(Box::new(StatusCode::NOT_FOUND))
            }
        }
        Err(error) => Ok(Box::new(
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(error.to_string()),
        )),
    }
}

#[utoipa::path(
    get,
    tag = "WCF",
    path = "/islogin",
    responses(
        (status = 200, body = ApiResponseBool, description = "查询微信登录状态")
    )
)]
pub async fn is_login(wechat: Arc<Mutex<WeChat>>) -> Result<Json, Infallible> {
    let wechat = wechat.lock().unwrap();
    let rsp = match wechat.clone().is_login() {
        Ok(status) => ApiResponse {
            status: 0,
            error: None,
            data: Some(status),
        },
        Err(error) => ApiResponse {
            status: 1,
            error: Some(error.to_string()),
            data: None,
        },
    };
    Ok(warp::reply::json(&rsp))
}

#[utoipa::path(
    get,
    tag = "WCF",
    path = "/selfwxid",
    responses(
        (status = 200, body = ApiResponseString, description = "返回登录账户 wxid")
    )
)]
pub async fn get_self_wxid(wechat: Arc<Mutex<WeChat>>) -> Result<Json, Infallible> {
    let wechat = wechat.lock().unwrap();
    let rsp = match wechat.clone().get_self_wxid() {
        Ok(wxid) => ApiResponse {
            status: 0,
            error: None,
            data: Some(wxid),
        },
        Err(error) => ApiResponse {
            status: 1,
            error: Some(error.to_string()),
            data: None,
        },
    };
    Ok(warp::reply::json(&rsp))
}
