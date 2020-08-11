use serde::{Serialize, Deserialize};
use anyhow::Result;

#[derive(Serialize, Deserialize)]
pub struct Variant {
    pub id: i32,
    pub name: String,
}

pub async fn add_variants(
    pool: &crate::DbAdminPool,
    variants: &Vec<Variant>,
) -> Result<()> {
    let mut tx = pool.0.begin().await?;
    for variant in variants {
        sqlx::query!(
            "INSERT INTO variants (
                site_variant_id
              , name
            ) VALUES (
                $1
              , $2
            )",
            variant.id,
            variant.name,
        ).execute(&mut tx).await?;
    }
    tx.commit().await?;
    Ok(())
}
