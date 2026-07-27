use rusqlite::{OptionalExtension, params};

use crate::store::ServerStore;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct StoredRouteManifest {
    pub(crate) repository_id: String,
    pub(crate) schema_version: String,
    pub(crate) manifest_json: String,
    pub(crate) content_sha256: String,
    pub(crate) publisher_id: String,
    pub(crate) published_at: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RouteManifestPublication {
    pub(crate) manifest: StoredRouteManifest,
    pub(crate) unchanged: bool,
}

impl ServerStore {
    pub(crate) fn publish_route_manifest(
        &self,
        repository_id: &str,
        schema_version: &str,
        manifest_json: &str,
        content_sha256: &str,
        publisher_id: &str,
        published_at: &str,
    ) -> rusqlite::Result<RouteManifestPublication> {
        if let Some(existing) = self.get_route_manifest(repository_id)? {
            if existing.content_sha256 == content_sha256 {
                return Ok(RouteManifestPublication {
                    manifest: existing,
                    unchanged: true,
                });
            }
        }

        self.conn.execute(
            "INSERT INTO repository_route_manifests (
                repository_id, schema_version, manifest_json, content_sha256,
                publisher_id, published_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(repository_id) DO UPDATE SET
                schema_version = excluded.schema_version,
                manifest_json = excluded.manifest_json,
                content_sha256 = excluded.content_sha256,
                publisher_id = excluded.publisher_id,
                published_at = excluded.published_at",
            params![
                repository_id,
                schema_version,
                manifest_json,
                content_sha256,
                publisher_id,
                published_at,
            ],
        )?;

        Ok(RouteManifestPublication {
            manifest: StoredRouteManifest {
                repository_id: repository_id.to_owned(),
                schema_version: schema_version.to_owned(),
                manifest_json: manifest_json.to_owned(),
                content_sha256: content_sha256.to_owned(),
                publisher_id: publisher_id.to_owned(),
                published_at: published_at.to_owned(),
            },
            unchanged: false,
        })
    }

    pub(crate) fn get_route_manifest(
        &self,
        repository_id: &str,
    ) -> rusqlite::Result<Option<StoredRouteManifest>> {
        self.conn
            .query_row(
                "SELECT repository_id, schema_version, manifest_json,
                        content_sha256, publisher_id, published_at
                 FROM repository_route_manifests
                 WHERE repository_id = ?1",
                [repository_id],
                |row| {
                    Ok(StoredRouteManifest {
                        repository_id: row.get(0)?,
                        schema_version: row.get(1)?,
                        manifest_json: row.get(2)?,
                        content_sha256: row.get(3)?,
                        publisher_id: row.get(4)?,
                        published_at: row.get(5)?,
                    })
                },
            )
            .optional()
    }
}
