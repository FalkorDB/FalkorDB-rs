use std::ffi::CStr;
use std::os::raw::c_char;
use std::path::PathBuf;
use tantivy::schema::{Schema, SchemaBuilder, INDEXED, STORED, TEXT};
use tantivy::Index;

type _SchemaBuilder = *mut SchemaBuilder;
type _IndexSchema = *mut Schema;
type _IndexRS = *mut Index;

#[no_mangle]
pub extern "C" fn schema_builder_new() -> _SchemaBuilder {
    let builder = Box::new(SchemaBuilder::new());
    Box::into_raw(builder)
}

#[no_mangle]
pub extern "C" fn schema_builder_add_text_field(
    builder_ptr: _SchemaBuilder,
    field_name: *const c_char,
    stored: bool,
) {
    let builder = unsafe {
        assert!(!builder_ptr.is_null());
        &mut *(builder_ptr as *mut SchemaBuilder)
    };

    let field_name = unsafe {
        assert!(!field_name.is_null());
        CStr::from_ptr(field_name).to_str().unwrap()
    };

    let options = if stored { TEXT | STORED } else { TEXT };
    builder.add_text_field(field_name, options);
}

#[no_mangle]
pub extern "C" fn schema_builder_add_numeric_field(
    builder_ptr: _SchemaBuilder,
    field_name: *const c_char
) {
    let builder = unsafe {
        assert!(!builder_ptr.is_null());
        &mut *(builder_ptr as *mut SchemaBuilder)
    };

    let field_name = unsafe {
        assert!(!field_name.is_null());
        CStr::from_ptr(field_name).to_str().unwrap()
    };

    builder.add_f64_field(field_name, INDEXED);
}

#[no_mangle]
pub extern "C" fn schema_builder_build(builder_ptr: _SchemaBuilder) -> _IndexSchema {
    let builder = unsafe {
        assert!(!builder_ptr.is_null());
        Box::from_raw(builder_ptr as *mut SchemaBuilder)
    };

    let schema = builder.build();
    let schema = Box::new(schema);
    Box::into_raw(schema)
}

#[no_mangle]
pub extern "C" fn schema_free(schema_ptr: _IndexSchema) {
    unsafe {
        if !schema_ptr.is_null() {
            let schema: Box<Schema> = Box::from_raw(schema_ptr as *mut Schema);
            drop(schema) // Explicitly drop the schema to free memory
        }
    }
}

#[no_mangle]
pub extern "C" fn create_index(schema_ptr: _IndexSchema, index_path: *const c_char, in_memory: bool) -> _IndexRS {
    let schema = unsafe {
        assert!(!schema_ptr.is_null());
        &*(schema_ptr as *mut Schema)
    };

    let index = if in_memory {
        // Create an in-memory index
        Index::create_in_ram(schema.clone())
    } else {
        // Create a disk-based index
        let index_path = unsafe {
            assert!(!index_path.is_null());
            CStr::from_ptr(index_path).to_str().unwrap()
        };
        let index_path = PathBuf::from(index_path);
        Index::create_in_dir(index_path, schema.clone()).expect("Failed to create disk-based index")
    };

    Box::into_raw(Box::new(index))
}

#[no_mangle]
pub extern "C" fn index_free(index_ptr: _IndexRS) {
    unsafe {
        if !index_ptr.is_null() {
            let _index: Box<Index> = Box::from_raw(index_ptr as *mut Index);
            // Index is dropped here
        }
    }
}
