#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

include!(concat!(env!("OUT_DIR"), "/sqlite3ext.rs"));

#[no_mangle]
    pub static mut sqlite3_api: *const sqlite3_api_routines = std::ptr::null();

#[no_mangle]
pub unsafe extern "C" fn sqlite3_extension_init(
		db: *mut sqlite3,
		pzErrMsg: *mut *mut ::std::os::raw::c_char,
		pApi: *const sqlite3_api_routines,
		) -> ::std::os::raw::c_int {
	sqlite3_api = pApi;
	return SQLITE_OK as ::std::os::raw::c_int;
}
