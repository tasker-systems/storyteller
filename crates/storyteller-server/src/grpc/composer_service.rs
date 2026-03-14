//! gRPC ComposerService implementation.
//!
//! Wraps [`storyteller_composer::SceneComposer`] catalog queries with proto
//! type conversion for the `ComposerService` gRPC service.

use std::sync::Arc;

use tonic::{Request, Response, Status};

use storyteller_composer::SceneComposer;

use crate::proto::composer_service_server::ComposerService;
use crate::proto::{
    ArchetypeInfo, ArchetypeList, DynamicInfo, DynamicsList, DynamicsRequest, GenreInfo, GenreList,
    GenreRequest, NameList, ProfileInfo, ProfileList, SettingList,
};

/// gRPC implementation of the `ComposerService` proto service.
///
/// All RPCs are read-only catalog queries delegated to [`SceneComposer`].
/// The composer is shared via `Arc` — no per-request allocation.
#[derive(Debug)]
pub struct ComposerServiceImpl {
    composer: Arc<SceneComposer>,
}

impl ComposerServiceImpl {
    /// Construct from a shared `SceneComposer`.
    pub fn new(composer: Arc<SceneComposer>) -> Self {
        Self { composer }
    }
}

#[tonic::async_trait]
impl ComposerService for ComposerServiceImpl {
    async fn list_genres(&self, _request: Request<()>) -> Result<Response<GenreList>, Status> {
        let genres = self.composer.genres();
        let genre_infos = genres
            .into_iter()
            .map(|g| GenreInfo {
                entity_id: g.entity_id,
                slug: g.id,
                display_name: g.display_name,
                description: g.description,
                archetype_count: g.archetype_count as u32,
                profile_count: g.profile_count as u32,
                dynamic_count: g.dynamic_count as u32,
            })
            .collect();
        Ok(Response::new(GenreList {
            genres: genre_infos,
        }))
    }

    async fn get_profiles_for_genre(
        &self,
        request: Request<GenreRequest>,
    ) -> Result<Response<ProfileList>, Status> {
        let genre_id = &request.get_ref().genre_id;
        let profiles = self.composer.profiles_for_genre(genre_id);
        let profile_infos = profiles
            .into_iter()
            .map(|p| ProfileInfo {
                entity_id: p.entity_id,
                slug: p.id,
                display_name: p.display_name,
                description: p.description,
                scene_type: p.scene_type,
                tension_min: p.tension_min,
                tension_max: p.tension_max,
                cast_size_min: p.cast_size_min as u32,
                cast_size_max: p.cast_size_max as u32,
            })
            .collect();
        Ok(Response::new(ProfileList {
            profiles: profile_infos,
        }))
    }

    async fn get_archetypes_for_genre(
        &self,
        request: Request<GenreRequest>,
    ) -> Result<Response<ArchetypeList>, Status> {
        let genre_id = &request.get_ref().genre_id;
        let archetypes = self.composer.archetypes_for_genre(genre_id);
        let archetype_infos = archetypes
            .into_iter()
            .map(|a| ArchetypeInfo {
                entity_id: a.entity_id,
                slug: a.id,
                display_name: a.display_name,
                description: a.description,
            })
            .collect();
        Ok(Response::new(ArchetypeList {
            archetypes: archetype_infos,
        }))
    }

    async fn get_dynamics_for_genre(
        &self,
        request: Request<DynamicsRequest>,
    ) -> Result<Response<DynamicsList>, Status> {
        let req = request.get_ref();
        let dynamics = self
            .composer
            .dynamics_for_genre(&req.genre_id, &req.selected_archetype_ids);
        let dynamic_infos = dynamics
            .into_iter()
            .map(|d| DynamicInfo {
                entity_id: d.entity_id,
                slug: d.id,
                display_name: d.display_name,
                description: d.description,
                role_a: d.role_a,
                role_b: d.role_b,
            })
            .collect();
        Ok(Response::new(DynamicsList {
            dynamics: dynamic_infos,
        }))
    }

    async fn get_names_for_genre(
        &self,
        request: Request<GenreRequest>,
    ) -> Result<Response<NameList>, Status> {
        let genre_id = &request.get_ref().genre_id;
        let names = self.composer.names_for_genre(genre_id);
        Ok(Response::new(NameList { names }))
    }

    async fn get_settings_for_genre(
        &self,
        request: Request<GenreRequest>,
    ) -> Result<Response<SettingList>, Status> {
        let _genre_id = &request.get_ref().genre_id;
        // TODO: Expose settings from DescriptorSet
        Ok(Response::new(SettingList { settings: vec![] }))
    }
}
