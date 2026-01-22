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

        let path_lower = path.to_lowercase().replace('\\', "/");
        // We use lowercase for matching, but keep original for replacement if needed (though we construct new path)

        let from_id_str = format!("skin{:02}", from_skin);

        if !path_lower.contains(&from_id_str) {
            return None;
        }

        // Target skin naming: skin00 for folders, "base" for filenames when to_skin is 0
        let to_folder_str = format!("skin{:02}", to_skin);
        let to_filename_str = if to_skin == 0 {
            "base".to_string()
        } else {
            format!("skin{:02}", to_skin)
        };

        // Construct regex-like replacements using simple string ops for robustness
        // We want to replace "skinXX" with "skinYY" in the folder structure
        // AND "champion_skinXX" with "champion_base" in the filename

        // 1. Path Replacement (Case insensitive by rebuilding)
        // We convert the path to parts? Or just replace on the lowercased path and lowercase everything?
        // League usually handles lowercase fine. Let's try safe lowercase conversion for output.

        // Replace folder path segment
        // "skins/skin05/" -> "skins/skin00/"
        let new_path = path_lower.replace(
            &format!("skins/{}/", from_id_str),
            &format!("skins/{}/", to_folder_str),
        );

        // Replace filename pattern: champion_skinXX -> champion_base or champion_skinYY
        // Logic: champion_skin05 -> champion_base
        let champion_lower = champion.to_lowercase();
        let final_path = new_path.replace(
            &format!("{}_{}", champion_lower, from_id_str),
            &format!("{}_{}", champion_lower, to_filename_str),
        );

        if final_path != path_lower {
            // Capitalize first letter of path parts if we want to be "nice", but usually not strictly required.
            // But let's return the lowercase path which is safe.
            Some(final_path)
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
