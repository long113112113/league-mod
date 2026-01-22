pub mod remapper;

use remapper::Remapper;

pub struct SwapEngine {
    remapper: Remapper,
}

impl SwapEngine {
    pub fn new() -> Self {
        Self {
            remapper: Remapper::new(),
        }
    }

    /// Generates a mapping of source file to destination file for a skin swap.
    ///
    /// # Arguments
    /// * `champion` - The champion name (e.g., "Ahri")
    /// * `from_skin` - The skin ID associated with the input files (e.g., 5)
    /// * `to_skin` - The target skin ID to swap to (usually 0 for base)
    /// * `file_path` - The relative path of the file (e.g., "ASSETS/Characters/Ahri/Skins/Skin05/Ahri_Skin05.skn")
    ///
    /// # Returns
    /// * `Option<String>` - The new remapped path if applicable, or None if no remapping needed.
    pub fn remap_path(
        &self,
        champion: &str,
        from_skin: u32,
        to_skin: u32,
        file_path: &str,
    ) -> Option<String> {
        self.remapper.remap(champion, from_skin, to_skin, file_path)
    }
}

/// Helper function to remap a path without instantiating SwapEngine.
pub fn remap_path(champion: &str, from_skin: u32, to_skin: u32, file_path: &str) -> Option<String> {
    let engine = SwapEngine::new();
    engine.remap_path(champion, from_skin, to_skin, file_path)
}
