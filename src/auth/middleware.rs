use axum::{
    extract::{Request, State},
    http::{HeaderMap, StatusCode, Uri, Method},
    middleware::Next,
    response::Response,
};
use std::sync::Arc;
use tracing::{debug, error, warn};

use super::{AuthContext, AuthzService, actions};
use crate::error::{ConfluxError, Result};

/// 授权中间件
/// 
/// 负责检查用户是否有权限访问特定资源
#[derive(Clone)]
pub struct AuthzMiddleware {
    authz_service: Arc<AuthzService>,
}

impl AuthzMiddleware {
    /// 创建新的授权中间件
    pub fn new(authz_service: Arc<AuthzService>) -> Self {
        Self { authz_service }
    }
}

/// Axum授权中间件函数
///
/// 这个函数会被注册为Axum的中间件，在每个请求处理前进行权限检查
pub async fn authz_middleware(
    State(authz_service): State<Arc<AuthzService>>,
    mut request: Request,
    next: Next,
) -> std::result::Result<Response, StatusCode> {
    let method = request.method().clone();
    let uri = request.uri().clone();
    let headers = request.headers().clone();

    // 检查是否为公共端点
    if is_public_endpoint(uri.path()) {
        debug!("Public endpoint accessed: {}", uri.path());
        return Ok(next.run(request).await);
    }

    // 提取认证信息
    let auth_context = match extract_auth_context(&headers) {
        Ok(ctx) => ctx,
        Err(e) => {
            warn!("Authentication failed for {}: {}", uri.path(), e);
            return Err(StatusCode::UNAUTHORIZED);
        }
    };

    // 解析资源和操作
    let (resource, action) = match parse_resource_and_action(&method, &uri) {
        Ok((res, act)) => (res, act),
        Err(e) => {
            error!("Failed to parse resource and action: {}", e);
            return Err(StatusCode::BAD_REQUEST);
        }
    };

    // 执行权限检查
    match authz_service
        .check(&auth_context.user_id, &auth_context.tenant_id, &resource, &action)
        .await
    {
        Ok(true) => {
            debug!(
                "Permission granted: user={}, tenant={}, resource={}, action={}",
                auth_context.user_id, auth_context.tenant_id, resource, action
            );
            
            // 将认证上下文添加到请求扩展中，供后续处理器使用
            request.extensions_mut().insert(auth_context);
            
            Ok(next.run(request).await)
        }
        Ok(false) => {
            warn!(
                "Permission denied: user={}, tenant={}, resource={}, action={}",
                auth_context.user_id, auth_context.tenant_id, resource, action
            );
            Err(StatusCode::FORBIDDEN)
        }
        Err(e) => {
            error!("Permission check error: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// 从请求头中提取认证上下文
/// 
/// 目前是一个简化的实现，在实际项目中应该验证JWT token
fn extract_auth_context(headers: &HeaderMap) -> Result<AuthContext> {
    // 检查Authorization头
    let auth_header = headers
        .get("authorization")
        .ok_or_else(|| ConfluxError::AuthError("Missing authorization header".to_string()))?;

    let auth_str = auth_header
        .to_str()
        .map_err(|_| ConfluxError::AuthError("Invalid authorization header".to_string()))?;

    if !auth_str.starts_with("Bearer ") {
        return Err(ConfluxError::AuthError("Invalid authorization format".to_string()));
    }

    let token = &auth_str[7..]; // 移除 "Bearer " 前缀

    // TODO: 在实际实现中，这里应该验证JWT token并提取用户信息
    // 现在我们使用一个简化的实现，假设token格式为 "user_id:tenant_id"
    let parts: Vec<&str> = token.split(':').collect();
    if parts.len() != 2 {
        return Err(ConfluxError::AuthError(
            "Invalid token format, expected user_id:tenant_id".to_string(),
        ));
    }

    let user_id = parts[0].to_string();
    let tenant_id = parts[1].to_string();

    if user_id.is_empty() || tenant_id.is_empty() {
        return Err(ConfluxError::AuthError("Empty user_id or tenant_id".to_string()));
    }

    Ok(AuthContext::new(user_id, tenant_id))
}

/// 解析请求的资源路径和操作类型
fn parse_resource_and_action(method: &Method, uri: &Uri) -> Result<(String, String)> {
    let path = uri.path();
    
    // 根据HTTP方法确定操作类型
    let action = match method {
        &Method::GET => actions::READ,
        &Method::POST => actions::WRITE,
        &Method::PUT => actions::WRITE,
        &Method::PATCH => actions::WRITE,
        &Method::DELETE => actions::DELETE,
        _ => {
            return Err(ConfluxError::AuthError(format!(
                "Unsupported HTTP method: {}",
                method
            )));
        }
    };

    // 解析资源路径
    let resource = if path.starts_with("/api/v1/") {
        // 移除API版本前缀
        path.strip_prefix("/api/v1").unwrap_or(path).to_string()
    } else {
        path.to_string()
    };

    Ok((resource, action.to_string()))
}

/// 检查是否为公共端点（不需要认证）
fn is_public_endpoint(path: &str) -> bool {
    let public_paths = [
        "/health",
        "/ready",
        "/_cluster/status",
        "/metrics",
        "/api/v1/auth/login", // 登录端点
    ];

    public_paths.iter().any(|&public_path| {
        path == public_path || path.starts_with(&format!("{}/", public_path))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::{HeaderValue, Method, Uri};

    #[test]
    fn test_extract_auth_context() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "authorization",
            HeaderValue::from_static("Bearer user123:tenant456"),
        );

        let ctx = extract_auth_context(&headers).unwrap();
        assert_eq!(ctx.user_id, "user123");
        assert_eq!(ctx.tenant_id, "tenant456");
    }

    #[test]
    fn test_extract_auth_context_invalid() {
        let mut headers = HeaderMap::new();
        headers.insert("authorization", HeaderValue::from_static("Invalid"));

        assert!(extract_auth_context(&headers).is_err());
    }

    #[test]
    fn test_parse_resource_and_action() {
        let method = Method::GET;
        let uri: Uri = "/api/v1/tenants/tenant1/configs".parse().unwrap();

        let (resource, action) = parse_resource_and_action(&method, &uri).unwrap();
        assert_eq!(resource, "/tenants/tenant1/configs");
        assert_eq!(action, "read");

        let method = Method::POST;
        let uri: Uri = "/api/v1/tenants/tenant1/configs".parse().unwrap();

        let (resource, action) = parse_resource_and_action(&method, &uri).unwrap();
        assert_eq!(resource, "/tenants/tenant1/configs");
        assert_eq!(action, "write");
    }

    #[test]
    fn test_is_public_endpoint() {
        assert!(is_public_endpoint("/health"));
        assert!(is_public_endpoint("/ready"));
        assert!(is_public_endpoint("/api/v1/auth/login"));
        assert!(!is_public_endpoint("/api/v1/configs"));
    }
}
