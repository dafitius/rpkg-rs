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

use crate::runtime::resource::runtime_resource_id::RuntimeResourceID;
use regex::Regex;
use std::str::FromStr;
use thiserror::Error;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

static CONSOLE_TAG: &str = "pc";

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
        let rid = Self { uri };

        if !rid.is_valid() {
            return Err(ResourceIDError::InvalidFormat("".to_string()));
        };

        Ok(Self {
            uri: rid.uri.replace(format!("{}_", CONSOLE_TAG).as_str(), ""),
        })
    }
}

impl ResourceID {
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a derived ResourceID from a existing one. This nests the original ResourceID
    /// ```
    ///  use rpkg_rs::misc::resource_id::ResourceID;
    ///
    ///  let resource_id = ResourceID::from_str_checked("[assembly:/_pro/_test/usern/materialclasses/ball_of_water_b.materialclass].pc_fx").unwrap();
    ///  let derived = resource_id.create_derived("dx11", "mate");
    ///  assert_eq!(derived.resource_path(), "[[assembly:/_pro/_test/usern/materialclasses/ball_of_water_b.materialclass].fx](dx11).pc_mate");
    ///
    /// ```
    pub fn create_derived(&self, parameters: &str, extension: &str) -> ResourceID {
        let mut derived = format!("[{}]", self.uri);
        if !parameters.is_empty() {
            derived += format!("({})", parameters).as_str();
        }
        derived += ".";
        if !extension.is_empty() {
            derived += extension;
        }

        ResourceID { uri: derived }
    }

    /// Create a ResourceID with aspect parameters
    /// ```
    ///  use rpkg_rs::misc::resource_id::ResourceID;
    ///
    ///  let resource_id = ResourceID::from_str_checked("[assembly:/templates/aspectdummy.aspect].pc_entitytype").unwrap();
    ///  let sub_id_1 = ResourceID::from_str_checked("[assembly:/_pro/effects/geometry/water.prim].pc_entitytype").unwrap();
    ///  let sub_id_2 = ResourceID::from_str_checked("[modules:/zdisablecameracollisionaspect.class].entitytype").unwrap();
    ///
    ///  let aspect = resource_id.create_aspect(vec![&sub_id_1, &sub_id_2]);
    ///
    /// assert_eq!(aspect.resource_path(), "[assembly:/templates/aspectdummy.aspect]([assembly:/_pro/effects/geometry/water.prim].entitytype,[modules:/zdisablecameracollisionaspect.class].entitytype).pc_entitytype");
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
    pub fn resource_path(&self) -> String {
        let mut platform_uri = String::new();

        let dot = self.uri.rfind('.').unwrap();
        platform_uri.push_str(&self.uri[..=dot]);
        platform_uri.push_str("pc_");
        platform_uri.push_str(&self.uri[dot + 1..]);

        platform_uri
    }

    /// Get the base ResourceID within a derived ResourceID
    /// ```
    ///  use rpkg_rs::misc::resource_id::ResourceID;
    ///
    ///  let resource_id = ResourceID::from_str_checked("[[[assembly:/_pro/_test/usern/materialclasses/ball_of_water_b.materialclass].fx](dx11).mate](dx12).pc_mate").unwrap();
    ///
    ///  let inner_most_path = resource_id.inner_most_resource_path();
    ///
    /// assert_eq!(inner_most_path.resource_path(), "[assembly:/_pro/_test/usern/materialclasses/ball_of_water_b.materialclass].pc_fx");
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
    ///  use rpkg_rs::misc::resource_id::ResourceID;
    ///
    ///  let resource_id = ResourceID::from_str_checked("[[[assembly:/_pro/_test/usern/materialclasses/ball_of_water_b.materialclass].fx](dx11).mate](dx12).pc_mate").unwrap();
    ///  let inner_path = resource_id.inner_resource_path();
    ///
    ///  assert_eq!(inner_path.resource_path(), "[[assembly:/_pro/_test/usern/materialclasses/ball_of_water_b.materialclass].fx](dx11).pc_mate");
    /// ```
    pub fn inner_resource_path(&self) -> ResourceID {
        let open_count = self.uri.chars().filter(|c| *c == '[').count();
        if open_count == 1 {
            return self.clone();
        }

        let re = Regex::new(r"\[(.*?)\][^]]*$").unwrap();
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
        let re = Regex::new(r"(.*)\((.*)\)\.(.*)").unwrap();
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
                && !self.uri.contains("unknown")
                && !self.uri.contains('*')
                && self.uri.contains(']')
        }
    }

    pub fn into_rrid(self) -> RuntimeResourceID {
        RuntimeResourceID::from_resource_id(&self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_parameters() {
        let mut resource_id = ResourceID::from_str_checked(
            "[assembly:/_pro/_test/usern/materialclasses/ball_of_water_b.materialclass].fx",
        )
        .unwrap();
        resource_id.add_parameter("lmao");
        assert_eq!(resource_id.resource_path(), "[assembly:/_pro/_test/usern/materialclasses/ball_of_water_b.materialclass](lmao).pc_fx");
        assert_eq!(resource_id.parameters(), ["lmao".to_string()]);

        resource_id.add_parameter("lmao2");
        assert_eq!(resource_id.resource_path(), "[assembly:/_pro/_test/usern/materialclasses/ball_of_water_b.materialclass](lmao,lmao2).pc_fx");
    }

    #[test]
    fn test_get_inner_most_resource_path() {
        let resource_id = ResourceID::from_str_checked(
            "[assembly:/_pro/_test/usern/materialclasses/ball_of_water_b.materialclass].fx",
        )
        .unwrap();
        let inner_path = resource_id.inner_most_resource_path();
        assert_eq!(
            inner_path.resource_path(),
            "[assembly:/_pro/_test/usern/materialclasses/ball_of_water_b.materialclass].pc_fx"
        );

        let resource_id = ResourceID::from_str_checked("[[assembly:/_pro/_test/usern/materialclasses/ball_of_water_b.materialclass].fx](dx11).mate").unwrap();
        let inner_path = resource_id.inner_most_resource_path();
        assert_eq!(
            inner_path.resource_path(),
            "[assembly:/_pro/_test/usern/materialclasses/ball_of_water_b.materialclass].pc_fx"
        );

        let resource_id = ResourceID::from_str_checked("[[[assembly:/_pro/_test/usern/materialclasses/ball_of_water_b.materialclass].fx](dx11).mate](dx12).pc_mate").unwrap();
        let inner_path = resource_id.inner_most_resource_path();
        assert_eq!(
            inner_path.resource_path(),
            "[assembly:/_pro/_test/usern/materialclasses/ball_of_water_b.materialclass].pc_fx"
        );
    }

    #[test]
    fn text_get_inner_resource_path() {
        let resource_id = ResourceID::from_str_checked(
            "[assembly:/_pro/_test/usern/materialclasses/ball_of_water_b.materialclass].fx",
        )
        .unwrap();
        let inner_path = resource_id.inner_resource_path();
        assert_eq!(
            inner_path.resource_path(),
            "[assembly:/_pro/_test/usern/materialclasses/ball_of_water_b.materialclass].pc_fx"
        );

        let resource_id = ResourceID::from_str_checked("[[assembly:/_pro/_test/usern/materialclasses/ball_of_water_b.materialclass].fx](dx11).mate").unwrap();
        let inner_path = resource_id.inner_resource_path();
        assert_eq!(
            inner_path.resource_path(),
            "[assembly:/_pro/_test/usern/materialclasses/ball_of_water_b.materialclass].pc_fx"
        );

        let resource_id = ResourceID::from_str_checked("[[[assembly:/_pro/_test/usern/materialclasses/ball_of_water_b.materialclass].fx](dx11).mate](dx12).pc_mate").unwrap();
        let inner_path = resource_id.inner_resource_path();
        assert_eq!(inner_path.resource_path(), "[[assembly:/_pro/_test/usern/materialclasses/ball_of_water_b.materialclass].fx](dx11).pc_mate");
    }
}
