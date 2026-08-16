use lopdf::{Dictionary, Document, Object, ObjectId};
use std::path::Path;

use crate::error::Excel2PdfError;

/// Merges multiple PDF files into a single PDF written to `output_path`.
pub fn merge_pdfs<P: AsRef<Path>>(input_paths: &[P], output_path: &Path) -> crate::Result<()> {
    if input_paths.is_empty() {
        return Err(Excel2PdfError::InvalidInput(
            "no input PDF files provided".into(),
        ));
    }

    // Load and renumber each document so their object IDs don't clash.
    let mut docs: Vec<Document> = Vec::with_capacity(input_paths.len());
    let mut next_start: u32 = 1;
    for p in input_paths {
        let mut doc = Document::load(p.as_ref()).map_err(|e| {
            Excel2PdfError::Pdf(format!("failed to load {}: {}", p.as_ref().display(), e))
        })?;
        doc.renumber_objects_with(next_start);
        next_start = doc.max_id + 1;
        docs.push(doc);
    }

    // Build the merged document.
    let mut merged = Document::with_version("1.5");

    // Collect page ObjectIds (in document order, then page order within doc).
    let mut all_page_ids: Vec<ObjectId> = Vec::new();
    for doc in &docs {
        let pages = doc.get_pages(); // BTreeMap<page_number, ObjectId>
        for page_id in pages.values() {
            all_page_ids.push(*page_id);
        }
    }

    // Copy all objects from each source document into merged.
    for doc in &docs {
        for (id, obj) in &doc.objects {
            merged.objects.insert(*id, obj.clone());
        }
    }

    // Allocate IDs for the new Pages node and Catalog.
    let pages_node_id: ObjectId = (next_start, 0);
    next_start += 1;
    let catalog_id: ObjectId = (next_start, 0);

    // Fix each Page's /Parent to point to our new Pages node.
    for page_id in &all_page_ids {
        if let Some(Object::Dictionary(dict)) = merged.objects.get_mut(page_id) {
            dict.set("Parent", Object::Reference(pages_node_id));
        }
    }

    // Build the Pages node.
    let mut pages_node = Dictionary::new();
    pages_node.set("Type", Object::Name(b"Pages".to_vec()));
    pages_node.set(
        "Kids",
        Object::Array(
            all_page_ids
                .iter()
                .map(|id| Object::Reference(*id))
                .collect(),
        ),
    );
    pages_node.set("Count", Object::Integer(all_page_ids.len() as i64));
    merged
        .objects
        .insert(pages_node_id, Object::Dictionary(pages_node));

    // Build the Catalog.
    let mut catalog = Dictionary::new();
    catalog.set("Type", Object::Name(b"Catalog".to_vec()));
    catalog.set("Pages", Object::Reference(pages_node_id));
    merged
        .objects
        .insert(catalog_id, Object::Dictionary(catalog));

    // Remove old Pages nodes (the top-level /Pages from each source doc)
    // so we don't have dangling tree nodes confusing readers.
    for doc in &docs {
        if let Ok(cat) = doc.catalog() {
            if let Ok(Object::Reference(old_pages_id)) = cat.get(b"Pages") {
                merged.objects.remove(old_pages_id);
            }
        }
    }

    // Remove old Catalog objects from source docs.
    for doc in &docs {
        if let Some(Object::Reference(cat_id)) = doc.trailer.get(b"Root").ok().cloned() {
            merged.objects.remove(&cat_id);
        }
    }

    merged.trailer.set("Root", Object::Reference(catalog_id));
    merged.max_id = catalog_id.0;

    merged
        .save(output_path)
        .map_err(|e| Excel2PdfError::Pdf(format!("failed to save merged PDF: {}", e)))?;

    Ok(())
}

/// Returns the number of pages in a PDF file.
pub fn page_count<P: AsRef<Path>>(pdf_path: P) -> crate::Result<u32> {
    let doc = Document::load(pdf_path.as_ref()).map_err(|e| {
        Excel2PdfError::Pdf(format!(
            "failed to load {}: {}",
            pdf_path.as_ref().display(),
            e
        ))
    })?;
    Ok(doc.get_pages().len() as u32)
}

/// Removes all pages after the first from the PDF at `pdf_path`.
/// If the file has one page or fewer, this is a no-op.
pub fn remove_all_but_first_page<P: AsRef<Path>>(pdf_path: P) -> crate::Result<()> {
    let path = pdf_path.as_ref();
    let mut doc = Document::load(path)
        .map_err(|e| Excel2PdfError::Pdf(format!("failed to load {}: {}", path.display(), e)))?;

    let count = doc.get_pages().len() as u32;
    if count <= 1 {
        return Ok(());
    }

    // Delete pages 2 through count (1-indexed page numbers).
    let to_delete: Vec<u32> = (2..=count).collect();
    doc.delete_pages(&to_delete);

    doc.save(path)
        .map_err(|e| Excel2PdfError::Pdf(format!("failed to save {}: {}", path.display(), e)))?;

    Ok(())
}
