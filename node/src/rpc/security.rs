//! Route access control (dev mode, admin token) and CORS.

use super::*;

/// Build the CORS policy. `"*"` allows any origin; otherwise only the listed
/// `http(s)://` origins are allowed. With no list, browsers are refused unless
/// the node runs in dev mode.
pub(super) fn build_cors(origins: &[String], dev_mode: bool) -> warp::cors::Cors {
    let base = warp::cors()
        .allow_methods(vec!["GET", "POST", "OPTIONS"])
        .allow_headers(vec!["Content-Type", "Authorization"])
        .max_age(3600);
    let allow_any = origins.iter().any(|o| o == "*") || (origins.is_empty() && dev_mode);
    if allow_any {
        return base.allow_any_origin().build();
    }
    let valid: Vec<&str> = origins
        .iter()
        .map(|o| o.trim_end_matches('/'))
        .filter(|o| o.starts_with("http://") || o.starts_with("https://"))
        .collect();
    base.allow_origins(valid).build()
}

/// Who may call the RPC routes that write state directly, bypassing consensus.
///
/// Those routes only exist for local devnets: outside `dev_mode` they answer
/// 404. When `admin_token` is set, admin routes also require
/// `Authorization: Bearer <token>`.
#[derive(Debug, Clone, Default)]
pub struct RpcSecurity {
    pub dev_mode: bool,
    pub admin_token: Option<String>,
    pub cors_origins: Vec<String>,
    /// Ed25519 seed of a genesis-funded dev account. The dev-mode faucet,
    /// bridge mint and WASM deploy routes submit real transactions signed by
    /// it, instead of writing state outside consensus.
    pub dev_funding_key: Option<[u8; 32]>,
}

#[derive(Debug)]
pub(super) struct Unauthorized;

impl warp::reject::Reject for Unauthorized {}

/// Compare without short-circuiting, so response timing does not leak how
/// much of the token a guess got right.
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

pub(super) fn bearer_matches(header: Option<&str>, token: &str) -> bool {
    header
        .and_then(|h| h.strip_prefix("Bearer "))
        .map(|given| constant_time_eq(given.trim().as_bytes(), token.as_bytes()))
        .unwrap_or(false)
}

/// Gate for dev-only routes that are safe to expose on a devnet without a
/// token (faucet, WASM deploy). 404 outside dev mode.
pub(super) fn dev_only(
    security: &RpcSecurity,
) -> impl Filter<Extract = (), Error = warp::Rejection> + Clone {
    let dev_mode = security.dev_mode;
    warp::any()
        .and_then(move || async move {
            if dev_mode {
                Ok(())
            } else {
                Err(warp::reject::not_found())
            }
        })
        .untuple_one()
}

/// Gate for admin routes (validator registration, delegation, runtime
/// upgrades, bridge relayer, peer dialing). 404 outside dev mode; 401 without
/// the admin token when one is configured.
pub(super) fn admin_only(
    security: &RpcSecurity,
) -> impl Filter<Extract = (), Error = warp::Rejection> + Clone {
    let dev_mode = security.dev_mode;
    let token = security.admin_token.clone();
    warp::header::optional::<String>("authorization")
        .and_then(move |auth: Option<String>| {
            let token = token.clone();
            async move {
                if !dev_mode {
                    return Err(warp::reject::not_found());
                }
                match token {
                    Some(t) if !bearer_matches(auth.as_deref(), &t) => {
                        Err(warp::reject::custom(Unauthorized))
                    }
                    _ => Ok(()),
                }
            }
        })
        .untuple_one()
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn status(
        filter: impl Filter<Extract = (), Error = warp::Rejection> + Clone + Send + Sync + 'static,
        auth: Option<&str>,
    ) -> u16 {
        let mut req = warp::test::request().path("/x");
        if let Some(a) = auth {
            req = req.header("authorization", a);
        }
        let route = warp::path("x")
            .and(filter)
            .map(warp::reply)
            .recover(handle_rejection);
        req.reply(&route).await.status().as_u16()
    }

    #[tokio::test]
    async fn state_writing_routes_are_absent_outside_dev_mode() {
        let prod = RpcSecurity::default();
        assert_eq!(status(admin_only(&prod), None).await, 404);
        assert_eq!(status(dev_only(&prod), None).await, 404);
        // A token does not unlock admin routes on a non-dev node.
        let prod_with_token = RpcSecurity {
            admin_token: Some("s3cret".into()),
            ..Default::default()
        };
        assert_eq!(
            status(admin_only(&prod_with_token), Some("Bearer s3cret")).await,
            404
        );
    }

    #[tokio::test]
    async fn admin_routes_require_the_token_when_set() {
        let dev = RpcSecurity {
            dev_mode: true,
            admin_token: Some("s3cret".into()),
            ..Default::default()
        };
        assert_eq!(status(admin_only(&dev), None).await, 401);
        assert_eq!(status(admin_only(&dev), Some("Bearer wrong")).await, 401);
        assert_eq!(status(admin_only(&dev), Some("s3cret")).await, 401);
        assert_eq!(status(admin_only(&dev), Some("Bearer s3cret")).await, 200);
        // Dev-only routes (faucet, WASM deploy) stay open in dev mode.
        assert_eq!(status(dev_only(&dev), None).await, 200);
    }

    #[tokio::test]
    async fn dev_mode_without_token_is_open() {
        let dev = RpcSecurity {
            dev_mode: true,
            ..Default::default()
        };
        assert_eq!(status(admin_only(&dev), None).await, 200);
    }

    #[test]
    fn token_comparison() {
        assert!(bearer_matches(Some("Bearer abc"), "abc"));
        assert!(!bearer_matches(Some("Bearer abd"), "abc"));
        assert!(!bearer_matches(Some("Bearer ab"), "abc"));
        assert!(!bearer_matches(None, "abc"));
    }
}
