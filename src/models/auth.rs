use super::exmo_jwt::ExmoJWT;
use models::*;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Auth {
    pub token: ExmoJWT,
    pub user_id: UserId,
}
