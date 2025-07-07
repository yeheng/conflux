use axum::{
    extract::Request,
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::Response,
};
use std::time::Instant;
use tracing::{debug, info, warn};

/// 请求日志中间件
pub async fn logging_middleware(request: Request, next: Next) -> Response {
    let start = Instant::now();
    let method = request.method().clone();
    let uri = request.uri().clone();
    let headers = request.headers().clone();

    // 提取客户端IP（如果有的话）
    let client_ip = extract_client_ip(&headers);

    debug!(
        "Incoming request: {} {} from {}",
        method,
        uri,
        client_ip.unwrap_or_else(|| "unknown".to_string())
    );

    // 处理请求
    let response = next.run(request).await;

    let duration = start.elapsed();
    let status = response.status();

    // 记录请求完成日志
    if status.is_success() {
        info!(
            "Request completed: {} {} -> {} in {:?}",
            method, uri, status, duration
        );
    } else if status.is_client_error() {
        warn!(
            "Client error: {} {} -> {} in {:?}",
            method, uri, status, duration
        );
    } else {
        warn!(
            "Server error: {} {} -> {} in {:?}",
            method, uri, status, duration
        );
    }

    response
}

/// 认证中间件（占位符实现）
/// 
/// 在后续的 Epic 中，这里会集成 JWT 验证和 RBAC 授权
pub async fn auth_middleware(request: Request, next: Next) -> Result<Response, StatusCode> {
    let headers = request.headers();

    // 检查 Authorization 头
    if let Some(auth_header) = headers.get("authorization") {
        if let Ok(auth_str) = auth_header.to_str() {
            if auth_str.starts_with("Bearer ") {
                // TODO: 在后续的 Epic 中实现 JWT 验证
                debug!("Authorization header found: {}", auth_str);
                
                // 暂时允许所有带有 Bearer token 的请求通过
                return Ok(next.run(request).await);
            }
        }
    }

    // 对于某些端点，我们允许匿名访问
    let path = request.uri().path();
    if is_public_endpoint(path) {
        debug!("Public endpoint accessed: {}", path);
        return Ok(next.run(request).await);
    }

    // 其他请求需要认证
    warn!("Unauthorized request to: {}", path);
    Err(StatusCode::UNAUTHORIZED)
}

/// 速率限制中间件（占位符实现）
/// 
/// 在后续的 Epic 中，这里会实现基于令牌桶或滑动窗口的速率限制
pub async fn rate_limit_middleware(request: Request, next: Next) -> Result<Response, StatusCode> {
    let client_ip = extract_client_ip(request.headers());
    
    // TODO: 实现实际的速率限制逻辑
    debug!(
        "Rate limit check for client: {}",
        client_ip.unwrap_or_else(|| "unknown".to_string())
    );

    // 暂时允许所有请求通过
    Ok(next.run(request).await)
}

/// 请求ID中间件
/// 
/// 为每个请求生成唯一的ID，用于链路追踪
pub async fn request_id_middleware(mut request: Request, next: Next) -> Response {
    let request_id = generate_request_id();
    
    // 将请求ID添加到请求头中
    request.headers_mut().insert(
        "x-request-id",
        request_id.parse().unwrap(),
    );

    debug!("Request ID generated: {}", request_id);

    let mut response = next.run(request).await;

    // 将请求ID添加到响应头中
    response.headers_mut().insert(
        "x-request-id",
        request_id.parse().unwrap(),
    );

    response
}

/// 提取客户端IP地址
fn extract_client_ip(headers: &HeaderMap) -> Option<String> {
    // 尝试从各种可能的头部提取客户端IP
    let ip_headers = [
        "x-forwarded-for",
        "x-real-ip",
        "x-client-ip",
        "cf-connecting-ip",
    ];

    for header_name in &ip_headers {
        if let Some(header_value) = headers.get(*header_name) {
            if let Ok(ip_str) = header_value.to_str() {
                // X-Forwarded-For 可能包含多个IP，取第一个
                let ip = ip_str.split(',').next().unwrap_or(ip_str).trim();
                if !ip.is_empty() {
                    return Some(ip.to_string());
                }
            }
        }
    }

    None
}

/// 检查是否为公共端点（不需要认证）
fn is_public_endpoint(path: &str) -> bool {
    let public_paths = [
        "/health",
        "/ready",
        "/_cluster/status",
        "/api/v1/fetch/configs", // 配置获取端点允许匿名访问
    ];

    public_paths.iter().any(|&public_path| {
        path == public_path || path.starts_with(&format!("{}/", public_path))
    })
}

/// 生成请求ID
fn generate_request_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    
    let timestamp = chrono::Utc::now().timestamp_millis() as u64;
    let counter = COUNTER.fetch_add(1, Ordering::Relaxed);
    
    format!("{:x}-{:x}", timestamp, counter)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::{HeaderValue};

    #[test]
    fn test_extract_client_ip() {
        let mut headers = HeaderMap::new();
        
        // 测试没有IP头的情况
        assert_eq!(extract_client_ip(&headers), None);
        
        // 测试 X-Forwarded-For 头
        headers.insert("x-forwarded-for", HeaderValue::from_static("192.168.1.1, 10.0.0.1"));
        assert_eq!(extract_client_ip(&headers), Some("192.168.1.1".to_string()));
        
        // 测试 X-Real-IP 头
        headers.clear();
        headers.insert("x-real-ip", HeaderValue::from_static("203.0.113.1"));
        assert_eq!(extract_client_ip(&headers), Some("203.0.113.1".to_string()));
    }

    #[test]
    fn test_is_public_endpoint() {
        assert!(is_public_endpoint("/health"));
        assert!(is_public_endpoint("/ready"));
        assert!(is_public_endpoint("/_cluster/status"));
        assert!(is_public_endpoint("/api/v1/fetch/configs/tenant/app/env/config"));
        
        assert!(!is_public_endpoint("/api/v1/configs/tenant/app/env/config/versions"));
        assert!(!is_public_endpoint("/api/v1/configs/tenant/app/env/config/releases"));
        assert!(!is_public_endpoint("/_cluster/nodes"));
    }

    #[test]
    fn test_generate_request_id() {
        let id1 = generate_request_id();
        let id2 = generate_request_id();
        
        // 确保生成的ID不同
        assert_ne!(id1, id2);
        
        // 确保ID格式正确（包含连字符）
        assert!(id1.contains('-'));
        assert!(id2.contains('-'));
    }

    #[tokio::test]
    async fn test_middleware_functions_exist() {
        // 这个测试只是确保中间件函数能够编译
        // 实际的功能测试需要更复杂的设置
        
        // 测试请求ID生成
        let request_id = generate_request_id();
        assert!(!request_id.is_empty());
        
        // 测试公共端点检查
        assert!(is_public_endpoint("/health"));
        assert!(!is_public_endpoint("/private"));
    }
}
