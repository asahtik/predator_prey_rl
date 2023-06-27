use bevy::{
    asset::{AssetLoader, LoadedAsset},
    reflect::TypeUuid,
};

use crate::helpers;

#[derive(TypeUuid)]
#[uuid = "7593c756-2b62-44cb-a763-1a345f16e779"]
pub struct MapAsset(pub helpers::map::Map<u8>);

#[derive(Default)]
pub struct MapLoader;

impl AssetLoader for MapLoader {
    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut bevy::asset::LoadContext,
    ) -> bevy::utils::BoxedFuture<'a, Result<(), bevy::asset::Error>> {
        Box::pin(async move {
            let custom_asset = MapAsset(helpers::map::Map::from_bytes(bytes));
            load_context.set_default_asset(LoadedAsset::new(custom_asset));
            Ok(())
        })
    }
    fn extensions(&self) -> &[&str] {
        &["map"]
    }
}
