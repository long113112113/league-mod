/// Handles path remapping for skin swapping.
pub struct Remapper;

impl Remapper {
    pub fn new() -> Self {
        Self
    }

    /// Remaps a file path from one skin ID to another.
    ///
    /// Handles two types of replacements:
    /// 1. Folder paths: `Skins/Skin05/` -> `Skins/Skin00/`
    /// 2. Filenames: `Champion_Skin05.ext` -> `Champion_Base.ext` (when to_skin is 0)
    pub fn remap(
        &self,
        champion: &str,
        from_skin: u32,
        to_skin: u32,
        path: &str,
    ) -> Option<String> {
        if from_skin == to_skin {
            return None;
        }

        let path = path.replace('\\', "/");
        let from_id_str = format!("Skin{:02}", from_skin);

        if !path.contains(&from_id_str) {
            return None;
        }

        // Target skin naming: Skin00 for folders, "Base" for filenames when to_skin is 0
        let to_folder_str = format!("Skin{:02}", to_skin);
        let to_filename_str = if to_skin == 0 {
            "Base".to_string()
        } else {
            format!("Skin{:02}", to_skin)
        };

        // Replace folder path segment
        let new_path = path.replace(
            &format!("Skins/{}/", from_id_str),
            &format!("Skins/{}/", to_folder_str),
        );

        // Replace filename pattern: Champion_SkinXX -> Champion_Base or Champion_SkinYY
        let new_path = new_path.replace(
            &format!("{}_{}", champion, from_id_str),
            &format!("{}_{}", champion, to_filename_str),
        );

        if new_path != path {
            Some(new_path)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_remap_skin05_to_base() {
        let remapper = Remapper::new();
        let champion = "Ahri";
        let src = "ASSETS/Characters/Ahri/Skins/Skin05/Ahri_Skin05.skn";

        let result = remapper.remap(champion, 5, 0, src);

        assert_eq!(
            result,
            Some("ASSETS/Characters/Ahri/Skins/Skin00/Ahri_Base.skn".to_string())
        );
    }

    #[test]
    fn test_remap_texture() {
        let remapper = Remapper::new();
        let champion = "Ahri";
        let src = "ASSETS/Characters/Ahri/Skins/Skin05/Ahri_Skin05_TX_CM.dds";

        let result = remapper.remap(champion, 5, 0, src);

        // Note: Skin00 usually implies "Base" in filenames for champions
        assert_eq!(
            result,
            Some("ASSETS/Characters/Ahri/Skins/Skin00/Ahri_Base_TX_CM.dds".to_string())
        );
    }

    #[test]
    fn test_remap_particles() {
        let remapper = Remapper::new();
        // Particles often don't follow the exact Champion_SkinXX pattern but reside in SkinXX folder
        let champion = "Ahri";
        let src = "ASSETS/Characters/Ahri/Skins/Skin05/Particles/Ahri_Skin05_Q_Orb.troy";

        let result = remapper.remap(champion, 5, 0, src);

        // Should replace folder Skin05->Skin00 AND filename Ahri_Skin05->Ahri_Base
        assert_eq!(
            result,
            Some("ASSETS/Characters/Ahri/Skins/Skin00/Particles/Ahri_Base_Q_Orb.troy".to_string())
        );
    }
}
