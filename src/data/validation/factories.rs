use super::super::settlement::DuelGraftFit;
use super::super::GameData;
use macroquad_toolkit::assets::TextureConfig;
use std::collections::HashSet;

pub(super) fn validate_factories(data: &GameData) -> Result<(), String> {
    for (id, factory) in data.factories.iter() {
        if factory.id != *id
            || data.world.region(&factory.region).is_none()
            || factory.floors.is_empty()
            || factory.heart_guard.is_empty()
            || factory.grow_cost <= 0
        {
            return Err(format!(
                "content validation: factory '{id}' has incomplete setup"
            ));
        }
        let mut floor_ids = HashSet::new();
        for (index, floor) in factory.floors.iter().enumerate() {
            if !floor_ids.insert(floor.id.as_str()) {
                return Err(format!(
                    "content validation: factory '{id}' has duplicate floor '{}'",
                    floor.id
                ));
            }
            if index + 1 == factory.floors.len() && floor.rows.iter().all(|row| !row.contains('H'))
            {
                return Err(format!(
                    "content validation: factory '{id}' has a heartless deepest floor"
                ));
            }
        }
        for species in &factory.grows {
            if !data.species.contains(species) {
                return Err(format!(
                    "content validation: factory '{id}' grows unknown species '{species}'"
                ));
            }
        }
        for unit in &factory.heart_guard {
            validate_unit_grafts(
                data,
                &format!("factory '{id}' guard"),
                &unit.species,
                &unit.grafts,
            )?;
        }
    }
    Ok(())
}

pub(super) fn validate_unit_grafts(
    data: &GameData,
    context: &str,
    species_id: &str,
    grafts: &[DuelGraftFit],
) -> Result<(), String> {
    let species = data.species.get(species_id).ok_or_else(|| {
        format!("content validation: {context} references unknown species '{species_id}'")
    })?;
    let mut mounts = HashSet::new();
    for graft in grafts {
        let limb = species.limb(&graft.limb).ok_or_else(|| {
            format!(
                "content validation: {context} references unknown limb '{}'",
                graft.limb
            )
        })?;
        let mount_class = limb.mounts.get(graft.slot).ok_or_else(|| {
            format!(
                "content validation: {context} references missing mount '{}'/{}",
                graft.limb, graft.slot
            )
        })?;
        if !mounts.insert((graft.limb.as_str(), graft.slot)) {
            return Err(format!(
                "content validation: {context} mounts two grafts at '{}'/{}",
                graft.limb, graft.slot
            ));
        }
        let def = data.graftware.get(&graft.graft).ok_or_else(|| {
            format!(
                "content validation: {context} references unknown graft '{}'",
                graft.graft
            )
        })?;
        if def.weight > *mount_class || species.power < def.min_power {
            return Err(format!(
                "content validation: {context} cannot fit graft '{}' on '{}'/{}",
                graft.graft, graft.limb, graft.slot
            ));
        }
    }
    Ok(())
}

pub(super) fn validate_textures(textures: &[TextureConfig]) -> Result<(), String> {
    let mut keys = HashSet::new();
    for texture in textures {
        if texture.key.trim().is_empty()
            || texture.path.trim().is_empty()
            || !keys.insert(texture.key.as_str())
        {
            return Err(format!(
                "content validation: texture manifest has duplicate or empty key '{}'",
                texture.key
            ));
        }
    }
    Ok(())
}
