//! Path identifier for a Glacier Resource file.
//!
//! ResourceID represents a resource identifier with utility methods for manipulating and extracting information from the identifier.
//! The identifier is expected to follow a specific format: ` [protocol:path/to/file.extension(parameters).platform_extension] `
//! The parameter can be optional. A ResourceID can also be nested/derived
//! ### Examples of valid ResourceID
//! ```txt
//! [assembly:/images/sprites/player.jpg](asspritesheet).pc_jpeg
//! [[assembly:/images/sprites/player.jpg](asspritesheet).pc_jpeg].pc_png
//! ```

use crate::resource::runtime_resource_id::{PlatformTag, RuntimeResourceID};
use lazy_regex::regex;
use std::str::FromStr;
use thiserror::Error;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Error, Debug)]
pub enum ResourceIDError {
    #[error("Invalid format {}", _0)]
    InvalidFormat(String),
}

#[derive(Clone, Debug, Default, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ResourceID {
    uri: String,
}

impl FromStr for ResourceID {
    type Err = ResourceIDError;

    fn from_str(source: &str) -> Result<Self, Self::Err> {
        let mut uri = source.to_ascii_lowercase();
        uri.retain(|c| c as u8 > 0x1F);
        let rid = Self { uri: uri.clone() };

        if !rid.is_valid() {
            return Err(ResourceIDError::InvalidFormat("".to_string()));
        };

        let agnostic_uri = if let Some(dot) = uri.rfind('.') {
            let left = &uri[..=dot];
            let right = &uri[dot + 1..];

            let mut out = String::with_capacity(uri.len());
            out.push_str(left);

            if let Some(underscore) = right.find('_') {
                out.push_str(&right[underscore + 1..]);
            } else {
                out.push_str(right);
            }

            out
        } else {
            uri
        };

        Ok(Self { uri: agnostic_uri })
    }
}

impl ResourceID {
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a derived ResourceID from a existing one. This nests the original ResourceID
    /// ```
    /// # use std::str::FromStr;
    /// # use rpkg_rs::misc::resource_id::ResourceID;
    /// # use rpkg_rs::misc::resource_id::ResourceIDError;
    /// # fn main() -> Result<(), ResourceIDError>{
    ///     let resource_id = ResourceID::from_str("[assembly:/_pro/_test/usern/materialclasses/ball_of_water_b.materialclass].pc_fx")?;
    ///     let derived = resource_id.create_derived("dx11", "mate");
    ///     assert_eq!(derived.resource_path_with_platform("pc"), "[[assembly:/_pro/_test/usern/materialclasses/ball_of_water_b.materialclass].fx](dx11).pc_mate");
    /// #   Ok(())
    /// # }
    /// ```
    pub fn create_derived(&self, parameters: &str, extension: &str) -> ResourceID {
        let mut derived = format!("[{}]", self.uri);
        if !parameters.is_empty() {
            derived += format!("({parameters})").as_str();
        }
        derived += ".";
        if !extension.is_empty() {
            derived += extension;
        }

        ResourceID { uri: derived }
    }

    /// Create a ResourceID with aspect parameters
    /// ```
    /// # use std::str::FromStr;
    /// # use rpkg_rs::misc::resource_id::ResourceID;     
    /// # use rpkg_rs::misc::resource_id::ResourceIDError;
    ///
    /// # fn main() -> Result<(), ResourceIDError>{
    ///  
    ///     let resource_id = ResourceID::from_str("[assembly:/templates/aspectdummy.aspect].pc_entitytype")?;
    ///     let sub_id_1 = ResourceID::from_str("[assembly:/_pro/effects/geometry/water.prim].pc_entitytype")?;
    ///     let sub_id_2 = ResourceID::from_str("[modules:/zdisablecameracollisionaspect.class].entitytype")?;
    ///
    ///     let aspect = resource_id.create_aspect(vec![&sub_id_1, &sub_id_2]);
    ///
    ///     assert_eq!(aspect.resource_path_with_platform("pc"), "[assembly:/templates/aspectdummy.aspect]([assembly:/_pro/effects/geometry/water.prim].entitytype,[modules:/zdisablecameracollisionaspect.class].entitytype).pc_entitytype");
    /// #   Ok(())
    /// # }
    ///
    /// ```
    pub fn create_aspect(&self, ids: Vec<&ResourceID>) -> ResourceID {
        let mut rid = self.clone();
        for id in ids {
            rid.add_parameter(id.uri.as_str());
        }
        rid
    }

    pub fn add_parameter(&mut self, param: &str) {
        let params = self.parameters();
        let new_uri = if params.is_empty() {
            match self.uri.rfind('.') {
                Some(index) => {
                    let mut modified_string = self.uri.to_string();
                    modified_string.insert(index, '(');
                    modified_string.insert_str(index + 1, param);
                    modified_string.insert(index + param.len() + 1, ')');
                    modified_string
                }
                None => self.uri.to_string(), // If no dot found, return the original string
            }
        } else {
            match self.uri.rfind(").") {
                Some(index) => {
                    let mut modified_string = self.uri.to_string();
                    modified_string.insert(index, ',');
                    modified_string.insert_str(index + 1, param);
                    modified_string
                }
                None => self.uri.to_string(), // If no dot found, return the original string
            }
        };
        self.uri = new_uri;
    }

    /// Get the resource path.
    /// Will append the platform tag
    #[deprecated(
        since = "1.4.0",
        note = "Use resource_path_with_platform(\"pc\") instead \
        resource_path() is not platform_agnostic and will always append the pc platform tag. \
        This function will be made platform agnostic in an upcoming release \
        Use uri() for the platform-agnostic form or resource_path_with_platform(...) for a platform specific form.\
        "
    )]
    pub fn resource_path(&self) -> String {
        self.resource_path_with_platform("pc")
    }

    pub fn uri(&self) -> &str {
        &self.uri
    }

    pub fn resource_path_with_platform(&self, platform_tag: &str) -> String {
        let mut platform_uri = String::new();

        if let Some(dot) = self.uri.rfind('.') {
            platform_uri.push_str(&self.uri[..=dot]);
            if !platform_tag.is_empty(){
                platform_uri.push_str(platform_tag);
                platform_uri.push('_');
            }
            platform_uri.push_str(&self.uri[dot + 1..]);
            platform_uri
        } else {
            self.uri.clone()
        }
    }

    /// Get the base ResourceID within a derived ResourceID
    /// ```
    /// # use std::str::FromStr;
    /// # use rpkg_rs::misc::resource_id::ResourceID;
    /// # use rpkg_rs::misc::resource_id::ResourceIDError;
    /// # fn main() -> Result<(), ResourceIDError>{
    ///     let resource_id = ResourceID::from_str("[[[assembly:/_pro/_test/usern/materialclasses/ball_of_water_b.materialclass].fx](dx11).mate](dx12).pc_mate")?;
    ///     let inner_most_path = resource_id.inner_most_resource_path();
    ///     assert_eq!(inner_most_path.uri(), "[assembly:/_pro/_test/usern/materialclasses/ball_of_water_b.materialclass].fx");
    /// #    Ok(())
    /// # }
    /// ```
    pub fn inner_most_resource_path(&self) -> ResourceID {
        let open_count = self.uri.chars().filter(|c| *c == '[').count();
        if open_count == 1 {
            return self.clone();
        }

        let parts = self.uri.splitn(open_count + 1, ']').collect::<Vec<&str>>();
        let rid_str = format!("{}]{}", parts[0], parts[1])
            .chars()
            .skip(open_count - 1)
            .collect::<String>();

        match Self::from_str(rid_str.as_str()) {
            Ok(r) => r,
            Err(_) => self.clone(),
        }
    }

    /// Get the base ResourceID within a derived ResourceID
    /// ```
    /// # use std::str::FromStr;
    /// # use rpkg_rs::misc::resource_id::ResourceID;
    /// # use rpkg_rs::misc::resource_id::ResourceIDError;
    /// # fn main() -> Result<(), ResourceIDError>{
    ///  
    ///     let resource_id = ResourceID::from_str("[[[assembly:/_pro/_test/usern/materialclasses/ball_of_water_b.materialclass].fx](dx11).mate](dx12).pc_mate")?;
    ///     let inner_path = resource_id.inner_resource_path();
    ///
    ///     assert_eq!(inner_path.resource_path_with_platform("pc"), "[[assembly:/_pro/_test/usern/materialclasses/ball_of_water_b.materialclass].fx](dx11).pc_mate");
    /// #   Ok(())
    /// }
    ///
    /// ```
    pub fn inner_resource_path(&self) -> ResourceID {
        let open_count = self.uri.chars().filter(|c| *c == '[').count();
        if open_count == 1 {
            return self.clone();
        }

        let re = regex!(r"\[(.*?)][^]]*$");
        if let Some(captures) = re.captures(&self.uri) {
            if let Some(inner_string) = captures.get(1) {
                if let Ok(rid) = ResourceID::from_str(inner_string.as_str()) {
                    return rid;
                }
            }
        }
        self.clone()
    }

    pub fn protocol(&self) -> Option<String> {
        match self.uri.find(':') {
            Some(n) => {
                let protocol: String = self.uri.chars().take(n).collect();
                Some(protocol.replace('[', ""))
            }
            None => None,
        }
    }

    pub fn parameters(&self) -> Vec<String> {
        let re = regex!(r"(.*)\((.*)\)\.(.*)");
        if let Some(captures) = re.captures(self.uri.as_str()) {
            if let Some(cap) = captures.get(2) {
                return cap
                    .as_str()
                    .split(',')
                    .map(|s: &str| s.to_string())
                    .collect();
            }
        }
        vec![]
    }

    pub fn path(&self) -> Option<String> {
        let path: String = self.uri.chars().skip(1).collect();
        if let Some(n) = path.rfind('/') {
            let p: String = path.chars().take(n).collect();
            if !p.contains('.') {
                return Some(p);
            }
        }
        None
    }

    pub fn is_empty(&self) -> bool {
        self.uri.is_empty()
    }

    pub fn is_valid(&self) -> bool {
        {
            self.uri.starts_with('[')
                && !self.uri.contains("unknown") //This isn't a good check :/
                && !self.uri.contains('*')
                && self.uri.contains(']')
        }
    }

    #[deprecated(
        since = "1.4.0",
        note = "into_rrid() hashes the ResourceID using PlatformTag::None. \
        Use into_rrid_with_platform(..., ..., PlatformTag::None) instead. \
        In a future release `into_rrid()` will require a runtime platform tag."
    )]
    pub fn into_rrid(self) -> RuntimeResourceID {
        RuntimeResourceID::from_resource_id_with_platform(&self, "", PlatformTag::None)
    }

    pub fn into_rrid_with_platform(self, resource_platform: &str, runtime_platform: PlatformTag) -> RuntimeResourceID {
        RuntimeResourceID::from_resource_id_with_platform(&self, resource_platform, runtime_platform)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_creation() -> Result<(), ResourceIDError> {

        let rid = ResourceID::from_str(
            "[assembly:/_PRO/Scenes/Missions/thefacility/vr_tutorial_pc_graduation.brick].entitytype",
        )?;
        assert_eq!(
            rid.uri(),
            "[assembly:/_pro/scenes/missions/thefacility/vr_tutorial_pc_graduation.brick].entitytype"
        );
        assert_eq!(
            rid.resource_path_with_platform("pc"),
            "[assembly:/_pro/scenes/missions/thefacility/vr_tutorial_pc_graduation.brick].pc_entitytype"
        );

        let rid = ResourceID::from_str(
            "[assembly:/_PRO/Scenes/Missions/thefacility/vr_tutorial_pc_graduation.brick].pc_entitytype",
        )?;
        assert_eq!(
            rid.uri(),
            "[assembly:/_pro/scenes/missions/thefacility/vr_tutorial_pc_graduation.brick].entitytype"
        );
        assert_eq!(
            rid.resource_path_with_platform("pc"),
            "[assembly:/_pro/scenes/missions/thefacility/vr_tutorial_pc_graduation.brick].pc_entitytype"
        );

        let rid = ResourceID::from_str("[assembly:/templates/aspectdummy.aspect].ps5_entitytype")?;
        assert_eq!(
            rid.uri(),
            "[assembly:/templates/aspectdummy.aspect].entitytype"
        );
        assert_eq!(
            rid.resource_path_with_platform("ps5"),
            "[assembly:/templates/aspectdummy.aspect].ps5_entitytype"
        );

        Ok(())
    }

    #[test]
    fn test_parameters_and_derived_ids() -> Result<(), ResourceIDError> {
        let mut resource_id = ResourceID::from_str(
            "[assembly:/_pro/_test/usern/materialclasses/ball_of_water_b.materialclass].pc_fx",
        )?;
        assert_eq!(
            resource_id.uri(),
            "[assembly:/_pro/_test/usern/materialclasses/ball_of_water_b.materialclass].fx"
        );

        resource_id.add_parameter("lmao");
        assert_eq!(
            resource_id.uri(),
            "[assembly:/_pro/_test/usern/materialclasses/ball_of_water_b.materialclass](lmao).fx"
        );
        assert_eq!(
            resource_id.resource_path_with_platform("pc"),
            "[assembly:/_pro/_test/usern/materialclasses/ball_of_water_b.materialclass](lmao).pc_fx"
        );
        assert_eq!(resource_id.parameters(), ["lmao".to_string()]);

        resource_id.add_parameter("lmao2");
        assert_eq!(
            resource_id.uri(),
            "[assembly:/_pro/_test/usern/materialclasses/ball_of_water_b.materialclass](lmao,lmao2).fx"
        );
        assert_eq!(
            resource_id.resource_path_with_platform("pc"),
            "[assembly:/_pro/_test/usern/materialclasses/ball_of_water_b.materialclass](lmao,lmao2).pc_fx"
        );

        let derived = resource_id.create_derived("dx11", "mate");
        assert_eq!(
            derived.uri(),
            "[[assembly:/_pro/_test/usern/materialclasses/ball_of_water_b.materialclass](lmao,lmao2).fx](dx11).mate"
        );
        assert_eq!(
            derived.resource_path_with_platform("pc"),
            "[[assembly:/_pro/_test/usern/materialclasses/ball_of_water_b.materialclass](lmao,lmao2).fx](dx11).pc_mate"
        );

        Ok(())
    }

    #[test]
    fn test_get_inner_most_resource_path() -> Result<(), ResourceIDError> {
        let resource_id = ResourceID::from_str(
            "[assembly:/_pro/_test/usern/materialclasses/ball_of_water_b.materialclass].fx",
        )?;
        let inner_path = resource_id.inner_most_resource_path();
        assert_eq!(
            inner_path.resource_path_with_platform("pc"),
            "[assembly:/_pro/_test/usern/materialclasses/ball_of_water_b.materialclass].pc_fx"
        );

        let resource_id = ResourceID::from_str(
            "[[assembly:/_pro/_test/usern/materialclasses/ball_of_water_b.materialclass].fx](dx11).mate",
        )?;
        let inner_path = resource_id.inner_most_resource_path();
        assert_eq!(
            inner_path.resource_path_with_platform("pc"),
            "[assembly:/_pro/_test/usern/materialclasses/ball_of_water_b.materialclass].pc_fx"
        );

        let resource_id = ResourceID::from_str(
            "[[[assembly:/_pro/_test/usern/materialclasses/ball_of_water_b.materialclass].fx](dx11).mate](dx12).pc_mate",
        )?;
        let inner_most = resource_id.inner_most_resource_path();
        let inner = resource_id.inner_resource_path();

        assert_eq!(
            inner_most.resource_path_with_platform("pc"),
            "[assembly:/_pro/_test/usern/materialclasses/ball_of_water_b.materialclass].pc_fx"
        );
        assert_eq!(
            inner.resource_path_with_platform("pc"),
            "[[assembly:/_pro/_test/usern/materialclasses/ball_of_water_b.materialclass].fx](dx11).pc_mate"
        );
        Ok(())
    }

    #[test]
    fn test_rrid_generation_is_agnostic_until_platform_is_explicit() -> Result<(), ResourceIDError>
    {
        let pc = ResourceID::from_str("[assembly:/templates/aspectdummy.aspect].pc_entitytype")?;
        let ps5 = ResourceID::from_str("[assembly:/templates/aspectdummy.aspect].ps5_entitytype")?;
        let ounce =
            ResourceID::from_str("[assembly:/templates/aspectdummy.aspect].ounce_entitytype")?;
        let plain = ResourceID::from_str("[assembly:/templates/aspectdummy.aspect].entitytype")?;

        assert_eq!(
            pc.uri(),
            "[assembly:/templates/aspectdummy.aspect].entitytype"
        );
        assert_eq!(
            ps5.uri(),
            "[assembly:/templates/aspectdummy.aspect].entitytype"
        );
        assert_eq!(
            ounce.uri(),
            "[assembly:/templates/aspectdummy.aspect].entitytype"
        );
        assert_eq!(
            plain.uri(),
            "[assembly:/templates/aspectdummy.aspect].entitytype"
        );

        assert_ne!(
            plain.clone().into_rrid_with_platform("", PlatformTag::Pc),
            plain.clone().into_rrid_with_platform("", PlatformTag::Ps5)
        );
        assert_ne!(
            plain.clone().into_rrid_with_platform("", PlatformTag::Ps5),
            plain.clone().into_rrid_with_platform("", PlatformTag::Ounce)
        );

        Ok(())
    }

    #[test]
    fn test_invalid_inputs() {
        assert!(ResourceID::from_str("not a resource id").is_err());
        assert!(ResourceID::from_str("unknown").is_err());
        assert!(ResourceID::from_str("[assembly:/foo/bar].*").is_err());
    }
}
