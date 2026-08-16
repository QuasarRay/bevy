use bevy::{ecs::reflect::AppTypeRegistry, prelude::*};

use super::{doc_skip_type_ids, should_skip_component};

/// Register a single ECS entity in the live scene document: create and link
/// its node, then upsert a patch for every serializable component. Ensures
/// the entity carries a stable `SceneNodeId` (adopting an existing one,
/// minting a fresh one otherwise) so the node keeps a cross-process
/// identity. Skips entities already in the document.
pub fn register_entity_in_ast(world: &mut World, entity: Entity) {
    let Some(doc) = world.get_resource::<jackdaw_bsn::SceneBsnAst>() else {
        return;
    };
    if doc.ast_for(entity).is_some() {
        return;
    }
    if world
        .get::<jackdaw_scene_types::SceneNodeId>(entity)
        .is_none()
        && let Ok(mut entity_mut) = world.get_entity_mut(entity)
    {
        entity_mut.insert(jackdaw_scene_types::SceneNodeId::next());
    }
    let doc = world.resource::<jackdaw_bsn::SceneBsnAst>();
    let parent = world
        .get::<ChildOf>(entity)
        .map(ChildOf::parent)
        .filter(|p| doc.ast_for(*p).is_some());
    jackdaw_bsn::create_entity_in_ast(world, entity, parent);

    let registry = world.resource::<AppTypeRegistry>().clone();
    let skip_ids = doc_skip_type_ids();
    let patches: Vec<(String, jackdaw_bsn::BsnPatch)> = {
        let reg = registry.read();
        let entity_ref = world.entity(entity);
        reg.iter()
            .filter(|registration| !skip_ids.contains(&registration.type_id()))
            .filter(|registration| {
                !should_skip_component(registration.type_info().type_path_table().path())
            })
            .filter_map(|registration| registration.data::<ReflectComponent>())
            .filter_map(|reflect_component| reflect_component.reflect(entity_ref))
            .filter_map(|component| {
                let type_path = component
                    .get_represented_type_info()?
                    .type_path()
                    .to_string();
                let patch =
                    jackdaw_bsn::component_to_bsn_patch(component.as_partial_reflect(), &reg);
                Some((type_path, patch))
            })
            .collect()
    };

    let Some(mut ast) = world.get_resource_mut::<jackdaw_bsn::SceneBsnAst>() else {
        return;
    };
    let Some(patches_entity) = ast.ast_for(entity) else {
        return;
    };
    for (type_path, patch) in patches {
        if let Some(existing) = ast.find_patch_by_type_path(patches_entity, &type_path) {
            ast.set_patch(existing, patch);
        } else {
            let patch_entity = ast.world.spawn(patch).id();
            if let Some(entity_patches) = ast.get_patches_mut(patches_entity) {
                entity_patches.0.push(patch_entity);
            }
        }
    }
}

/// Register multiple ECS entities in the live scene document.
pub fn register_entities_in_ast(world: &mut World, entities: &[Entity]) {
    for &entity in entities {
        register_entity_in_ast(world, entity);
    }
}
