use crate::core::production_document::ProductionDocument;
use crate::core::timeline::Project;

#[derive(serde::Serialize, serde::Deserialize)]
pub(crate) struct AutosaveSnapshot {
    pub(crate) project: Project,
    #[serde(default)]
    pub(crate) production_document: Option<ProductionDocumentMetadata>,
}

#[derive(serde::Serialize)]
pub(crate) struct AutosaveSnapshotRef<'a> {
    pub(crate) project: &'a Project,
    pub(crate) production_document: Option<&'a ProductionDocumentMetadata>,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub(crate) struct ProductionDocumentMetadata {
    pub(crate) schema_version: u32,
    pub(crate) color_lut_path: Option<String>,
    pub(crate) audio: crate::core::production_document::AudioDocumentSettings,
    pub(crate) tempo: crate::core::unified_time::TempoMap,
    pub(crate) bindings: Vec<crate::core::automation_binding::AutomationBinding>,
}

impl Default for ProductionDocumentMetadata {
    fn default() -> Self {
        Self {
            schema_version: ProductionDocument::CURRENT_SCHEMA_VERSION,
            color_lut_path: None,
            audio: crate::core::production_document::AudioDocumentSettings::default(),
            tempo: crate::core::unified_time::TempoMap::default(),
            bindings: Vec::new(),
        }
    }
}

impl From<&ProductionDocument> for ProductionDocumentMetadata {
    fn from(document: &ProductionDocument) -> Self {
        Self {
            schema_version: document.schema_version,
            color_lut_path: document.color_lut_path.clone(),
            audio: document.audio.clone(),
            tempo: document.tempo.clone(),
            bindings: document.bindings.clone(),
        }
    }
}

impl ProductionDocumentMetadata {
    pub(crate) fn into_document(self, project: Project) -> ProductionDocument {
        ProductionDocument {
            schema_version: self.schema_version,
            project,
            color_lut_path: self.color_lut_path,
            audio: self.audio,
            tempo: self.tempo,
            bindings: self.bindings,
        }
    }
}
