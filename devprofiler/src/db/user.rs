use sled::IVec;

use crate::db::config::get_db;
use crate::utils::user::User;

pub fn save_user_to_db(user: &User) {
    let db = get_db();
    let provider_obj = user.provider();
    let user_key = format!("{}/{}/{}", 
        provider_obj.provider_type().to_string(), user.workspace(), provider_obj.id());
    println!("user_key = {}", &user_key);
  
    // Serialize repo struct to JSON 
    let json = serde_json::to_vec(user).expect("Failed to serialize user");
  
    // Insert JSON into sled DB
    db.insert(IVec::from(user_key.as_bytes()), json).expect("Failed to upsert user into sled DB");
}
