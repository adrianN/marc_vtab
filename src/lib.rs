#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

include!(concat!(env!("OUT_DIR"), "/sqlite3ext.rs"));

#[no_mangle]
pub static mut sqlite3_api: *const sqlite3_api_routines = std::ptr::null();

#[repr(C)]
struct myvtab_vtab {
    base: sqlite3_vtab,
}

#[repr(C)]
struct myvtab_cursor {
    base: sqlite3_vtab_cursor,
    iRowId: i64,
}

#[no_mangle]
pub unsafe extern "C" fn myvtabConnect(
    db: *mut sqlite3,
    _pAux: *mut ::std::os::raw::c_void,
    _argc: ::std::os::raw::c_int,
    _argv: *const *const ::std::os::raw::c_char,
    ppVtab: *mut *mut sqlite3_vtab,
    _pzErr: *mut *mut ::std::os::raw::c_char,
) -> ::std::os::raw::c_int {
    let s = std::ffi::CString::new("CREATE TABLE x(a,b)").expect("Can't alloc string");
    let rc = sqlite3_declare_vtab(db, s.as_ptr());
    if rc == SQLITE_OK as i32 {
        let pvTab = sqlite3_malloc(std::mem::size_of::<myvtab_vtab>() as i32) as *mut myvtab_vtab;
        *ppVtab = pvTab as *mut sqlite3_vtab;
        if pvTab == std::ptr::null_mut() {
            return SQLITE_NOMEM as i32;
        }
        let newTab = myvtab_vtab {
            base: sqlite3_vtab {
                nRef: 0,
                pModule: std::ptr::null_mut(),
                zErrMsg: std::ptr::null_mut(),
            },
        };
        std::ptr::write(pvTab, newTab);
    }
    return rc;
}

#[no_mangle]
pub unsafe extern "C" fn sqlite3_extension_init(
    db: *mut sqlite3,
    pzErrMsg: *mut *mut ::std::os::raw::c_char,
    pApi: *const sqlite3_api_routines,
) -> ::std::os::raw::c_int {
    sqlite3_api = pApi;
    return SQLITE_OK as ::std::os::raw::c_int;
}
