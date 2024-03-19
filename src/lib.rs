#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use marclib::marcrecord::BufferedMarcReader;
use std::fs::File;
use std::os::raw::{c_char, c_int, c_void};

include!(concat!(env!("OUT_DIR"), "/sqlite3ext.rs"));

static mut sqlite3_api: *const sqlite3_api_routines = std::ptr::null();

#[repr(C)]
struct myvtab_vtab {
    base: sqlite3_vtab,
    vtabArgs : Option<VtabArgs>// we make it Option so that we can free it in Disconnect
}

#[repr(C)]
struct myvtab_cursor {
    base: sqlite3_vtab_cursor,
    reader: Option<BufferedMarcReader<File>>,
    iRowId: i32,
}

unsafe extern "C" fn myvtabCreate(
    db: *mut sqlite3,
    pAux: *mut c_void,
    argc: c_int,
    argv: *const *const c_char,
    ppVtab: *mut *mut sqlite3_vtab,
    pzErr: *mut *mut c_char,
) -> c_int {
    return myvtabConnect(db, pAux, argc, argv, ppVtab, pzErr);
}

struct VtabArgs {
  filename : String,
  fieldTypes : Vec<usize>
}

fn readVtabArgs(args : Vec<&str>) -> VtabArgs {
  let filename = args[0].to_owned();
  let fieldTypes : Vec<usize> = args[1..].iter().map(|x| x.parse::<usize>().expect("invalid field type")).collect::<Vec<usize>>();
  VtabArgs { filename : filename, fieldTypes : fieldTypes }
}

unsafe extern "C" fn myvtabConnect(
    db: *mut sqlite3,
    _pAux: *mut c_void,
    argc: c_int,
    argv: *const *const c_char,
    ppVtab: *mut *mut sqlite3_vtab,
    _pzErr: *mut *mut c_char,
) -> c_int {
    let s = std::ffi::CString::new("CREATE TABLE x(a,b)").expect("Can't alloc string");
    let rc = ((*sqlite3_api).declare_vtab).unwrap()(db, s.as_ptr());
    if rc == SQLITE_OK as i32 {
        let pvTab = ((*sqlite3_api).malloc).unwrap()(std::mem::size_of::<myvtab_vtab>() as i32)
            as *mut myvtab_vtab;
        *ppVtab = pvTab as *mut sqlite3_vtab;
        if pvTab == std::ptr::null_mut() {
            return SQLITE_NOMEM as i32;
        }
        if argc > 3 {
            let arguments = (3..argc).map(|i| std::ffi::CStr::from_ptr(*argv.offset(i as isize))
                .to_str()
                .expect("expect valid str")).collect();
            let vtabArgs = readVtabArgs(arguments);
            let newTab = myvtab_vtab {
                base: sqlite3_vtab {
                    nRef: 0,
                    pModule: std::ptr::null_mut(),
                    zErrMsg: std::ptr::null_mut(),
                },
                vtabArgs : Some(vtabArgs),
            };
            std::ptr::write(pvTab, newTab);
        } else {
            unimplemented!();
        }
    }
    return rc;
}

unsafe extern "C" fn myvtabDisconnect(pVtab: *mut sqlite3_vtab) -> c_int {
    let pMyVtab = pVtab as *mut myvtab_vtab;
    (*pMyVtab).vtabArgs = None;
    ((*sqlite3_api).free).unwrap()(pMyVtab as *mut c_void);
    return SQLITE_OK as c_int;
}

unsafe extern "C" fn myvtabOpen(
    p: *mut sqlite3_vtab,
    ppCursor: *mut *mut sqlite3_vtab_cursor,
) -> std::os::raw::c_int {
    eprintln!("size {}", std::mem::size_of::<myvtab_cursor>());
    let pCur = ((*sqlite3_api).malloc).unwrap()(std::mem::size_of::<myvtab_cursor>() as i32)
        as *mut myvtab_cursor;
    if pCur == std::ptr::null_mut() {
        return SQLITE_NOMEM as i32;
    };
    let pTab = p as *const myvtab_vtab;

    let filename = &(*pTab).vtabArgs.as_ref().unwrap().filename;
    let newCur = myvtab_cursor {
        base: sqlite3_vtab_cursor { pVtab: p },
        reader: Some(BufferedMarcReader::new(
            File::open(filename).expect("failed to open file"),
        )),
        iRowId: 0,
    };
    std::ptr::write(pCur, newCur);
    *ppCursor = &mut ((*pCur).base);
    return SQLITE_OK as i32;
}

unsafe extern "C" fn myvtabClose(cur: *mut sqlite3_vtab_cursor) -> c_int {
    let pMyCurser = cur as *mut myvtab_cursor;
    (*pMyCurser).reader = None;
    ((*sqlite3_api).free).unwrap()(cur as *mut c_void);
    return SQLITE_OK as i32;
}

unsafe extern "C" fn myvtabNext(cur: *mut sqlite3_vtab_cursor) -> c_int {
    let pMyVtab = cur as *mut myvtab_cursor;
    let success = (*pMyVtab).reader.as_mut().unwrap().advance();
    if success.is_ok() {
        (*pMyVtab).iRowId += 1;
        return SQLITE_OK as i32;
    } else {
        dbg!(success);
        return SQLITE_ERROR as i32;
    }
}

unsafe extern "C" fn myvtabColumn(
    cur: *mut sqlite3_vtab_cursor,
    ctx: *mut sqlite3_context,
    i: c_int,
) -> c_int {
    let pCur = cur as *mut myvtab_cursor;
    let pTab = (*pCur).base.pVtab as *const myvtab_vtab;
    let record = (*pCur).reader.as_ref().unwrap().get().unwrap();
    if i == 0 {
        ((*sqlite3_api).result_int).unwrap()(ctx, record.header().record_length() as i32);
    } else {
        ((*sqlite3_api).result_int).unwrap()(ctx, 2 * 1000 + (*pCur).iRowId);
    }
    return SQLITE_OK as i32;
}

unsafe extern "C" fn myvtabRowid(cur: *mut sqlite3_vtab_cursor, pRowId: *mut i64) -> c_int {
    let pCur = cur as *mut myvtab_cursor;
    *pRowId = (*pCur).iRowId as i64;
    return SQLITE_OK as i32;
}

unsafe extern "C" fn myvtabEof(cur: *mut sqlite3_vtab_cursor) -> c_int {
    let pCur = cur as *mut myvtab_cursor;
    if (*pCur).reader.as_ref().map(|x| x.is_eof()).unwrap_or(false) { return 1; }

    if (*pCur).iRowId >= 1000000 {
        return 1;
    } else {
        return 0;
    }
}

unsafe extern "C" fn myvtabFilter(
    cur: *mut sqlite3_vtab_cursor,
    idxNum: c_int,
    idxStr: *const c_char,
    argc: c_int,
    argv: *mut *mut sqlite3_value,
) -> c_int {
    let pCur = cur as *mut myvtab_cursor;
    (*pCur).iRowId = 1;
    (*pCur).reader.as_mut().unwrap().advance();
    return SQLITE_OK as i32;
}

unsafe extern "C" fn myvtabBestIndex(
    tab: *mut sqlite3_vtab,
    pIdxInfo: *mut sqlite3_index_info,
) -> c_int {
    (*pIdxInfo).estimatedCost = 10.0;
    (*pIdxInfo).estimatedRows = 10;
    return SQLITE_OK as i32;
}

pub static myvtabModule: sqlite3_module = sqlite3_module {
    iVersion: 1,
    //xCreate: None,
    xCreate: Some(myvtabCreate),
    xConnect: Some(myvtabConnect),
    xBestIndex: Some(myvtabBestIndex),
    xDisconnect: Some(myvtabDisconnect),
    xDestroy: Some(myvtabDisconnect),
    xOpen: Some(myvtabOpen),
    xClose: Some(myvtabClose),
    xFilter: Some(myvtabFilter),
    xNext: Some(myvtabNext),
    xEof: Some(myvtabEof),
    xColumn: Some(myvtabColumn),
    xRowid: Some(myvtabRowid),
    xUpdate: None,
    xBegin: None,
    xSync: None,
    xCommit: None,
    xRollback: None,
    xFindFunction: None,
    xRename: None,
    xSavepoint: None,
    xRelease: None,
    xRollbackTo: None,
    xShadowName: None,
    xIntegrity: None,
};

#[no_mangle]
pub unsafe extern "C" fn sqlite3_extension_init(
    db: *mut sqlite3,
    pzErrMsg: *mut *mut c_char,
    pApi: *const sqlite3_api_routines,
) -> c_int {
    sqlite3_api = pApi;
    let s = std::ffi::CString::new("myvtab").expect("Can't alloc string");
    let rc =
        (*sqlite3_api).create_module.unwrap()(db, s.as_ptr(), &myvtabModule, std::ptr::null_mut());
    return rc as c_int;
}
