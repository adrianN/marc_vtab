#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use marclib::marcrecord::{BufferedMarcReader, MarcRecordFieldIter};
use marclib::record::Record;
use marclib::record::RecordField;
use std::fs::File;
use std::io::Cursor;
use std::os::raw::{c_char, c_int, c_void};

include!(concat!(env!("OUT_DIR"), "/sqlite3ext.rs"));

static mut sqlite3_api: *const sqlite3_api_routines = std::ptr::null();

#[repr(C)]
struct marcvtab_vtab {
    base: sqlite3_vtab,
    vtabArgs: Option<VtabArgs>, // we make it Option so that we can free it in Disconnect
}

#[repr(C)]
struct marcvtab_cursor {
    base: sqlite3_vtab_cursor,
    reader: Option<BufferedMarcReader<File>>,
    iRowId: i32,
}

unsafe extern "C" fn marcvtabCreate(
    db: *mut sqlite3,
    pAux: *mut c_void,
    argc: c_int,
    argv: *const *const c_char,
    ppVtab: *mut *mut sqlite3_vtab,
    pzErr: *mut *mut c_char,
) -> c_int {
    return marcvtabConnect(db, pAux, argc, argv, ppVtab, pzErr);
}

struct VtabArgs {
    filename: String,
    fieldTypes: Vec<usize>,
}

fn readVtabArgs(args: Vec<&str>) -> VtabArgs {
    let mut filename = None;
    let mut fieldTypes = None;
    for arg in &args {
        if arg.starts_with("file") {
            filename = arg.rsplit('=').next().map(|x| x.trim());
        } else if arg.starts_with("fields") {
            if let Some(fieldList) = arg
                .rsplit('=')
                .next()
                .map(|x| x.trim())
                .and_then(|x| x.strip_prefix('\''))
                .and_then(|x| x.strip_suffix('\''))
            {
                fieldTypes = Some(
                    fieldList
                        .split(',')
                        .map(|x| x.trim().parse::<usize>().expect("invalid field type"))
                        .collect::<Vec<usize>>(),
                );
            } else {
                unimplemented!();
            }
        }
    }
    //let filename = args[0].to_owned();
    //    let fieldTypes: Vec<usize> = args[1..]
    //        .iter()
    //        .map(|x| x.parse::<usize>().expect("invalid field type"))
    //        .collect::<Vec<usize>>();
    VtabArgs {
        filename: filename.unwrap().to_owned(),
        fieldTypes: fieldTypes.unwrap(),
    }
}

fn createTableFromArgs(vtabArgs: &VtabArgs) -> String {
    let mut s = "CREATE TABLE x(".to_string();
    for fieldtype in &vtabArgs.fieldTypes {
        s += &format!("x{} BLOB, ", fieldtype);
    }
    s += "entry_length INT, full_record BLOB, field_types BLOB);";
    s
}

unsafe extern "C" fn marcvtabConnect(
    db: *mut sqlite3,
    _pAux: *mut c_void,
    argc: c_int,
    argv: *const *const c_char,
    ppVtab: *mut *mut sqlite3_vtab,
    _pzErr: *mut *mut c_char,
) -> c_int {
    let mut vtabArgs = None;
    if argc > 3 {
        let arguments = (3..argc)
            .map(|i| {
                std::ffi::CStr::from_ptr(*argv.offset(i as isize))
                    .to_str()
                    .expect("expect valid str")
            })
            .collect();
        vtabArgs = Some(readVtabArgs(arguments));
    } else {
        unimplemented!();
    }
    let s = std::ffi::CString::new(createTableFromArgs(vtabArgs.as_ref().unwrap()))
        .expect("Can't alloc string");
    let rc = ((*sqlite3_api).declare_vtab).unwrap()(db, s.as_ptr());
    if rc == SQLITE_OK as i32 {
        let pvTab = ((*sqlite3_api).malloc).unwrap()(std::mem::size_of::<marcvtab_vtab>() as i32)
            as *mut marcvtab_vtab;
        *ppVtab = pvTab as *mut sqlite3_vtab;
        if pvTab == std::ptr::null_mut() {
            return SQLITE_NOMEM as i32;
        }
        let newTab = marcvtab_vtab {
            base: sqlite3_vtab {
                nRef: 0,
                pModule: std::ptr::null_mut(),
                zErrMsg: std::ptr::null_mut(),
            },
            vtabArgs: vtabArgs,
        };
        std::ptr::write(pvTab, newTab);
    }
    return rc;
}

unsafe extern "C" fn marcvtabDisconnect(pVtab: *mut sqlite3_vtab) -> c_int {
    let pMyVtab = pVtab as *mut marcvtab_vtab;
    (*pMyVtab).vtabArgs = None;
    ((*sqlite3_api).free).unwrap()(pMyVtab as *mut c_void);
    return SQLITE_OK as c_int;
}

unsafe extern "C" fn marcvtabOpen(
    p: *mut sqlite3_vtab,
    ppCursor: *mut *mut sqlite3_vtab_cursor,
) -> std::os::raw::c_int {
    let pCur = ((*sqlite3_api).malloc).unwrap()(std::mem::size_of::<marcvtab_cursor>() as i32)
        as *mut marcvtab_cursor;
    if pCur == std::ptr::null_mut() {
        return SQLITE_NOMEM as i32;
    };
    let pTab = p as *const marcvtab_vtab;

    let filename = &(*pTab).vtabArgs.as_ref().unwrap().filename;
    let newCur = marcvtab_cursor {
        base: sqlite3_vtab_cursor { pVtab: p },
        reader: Some(BufferedMarcReader::new(
            File::open(filename).expect(&format!("failed to open file {}", filename)),
        )),
        iRowId: 0,
    };
    std::ptr::write(pCur, newCur);
    *ppCursor = &mut ((*pCur).base);
    return SQLITE_OK as i32;
}

unsafe extern "C" fn marcvtabClose(cur: *mut sqlite3_vtab_cursor) -> c_int {
    let pMyCurser = cur as *mut marcvtab_cursor;
    (*pMyCurser).reader = None;
    ((*sqlite3_api).free).unwrap()(cur as *mut c_void);
    return SQLITE_OK as i32;
}

unsafe extern "C" fn marcvtabNext(cur: *mut sqlite3_vtab_cursor) -> c_int {
    let pMyVtab = cur as *mut marcvtab_cursor;
    let success = (*pMyVtab).reader.as_mut().unwrap().advance();
    if success.is_ok() {
        (*pMyVtab).iRowId += 1;
        return SQLITE_OK as i32;
    } else {
        return SQLITE_ERROR as i32;
    }
}

unsafe extern "C" fn marcvtabColumn(
    cur: *mut sqlite3_vtab_cursor,
    ctx: *mut sqlite3_context,
    j: c_int,
) -> c_int {
    let pCur = cur as *mut marcvtab_cursor;
    let pTab = (*pCur).base.pVtab as *const marcvtab_vtab;
    let record = (*pCur).reader.as_ref().unwrap().get().unwrap();
    let field_types = &(*pTab).vtabArgs.as_ref().unwrap().fieldTypes;
    let i = j as usize;
    let ptr = usize::MAX as *mut c_void;
    let SQLITE_TRANSIENT =
        std::mem::transmute::<*mut c_void, unsafe extern "C" fn(arg1: *mut c_void)>(ptr);
    if i < field_types.len() {
        let iter = MarcRecordFieldIter::new(&record, Some(field_types[i]));
        let fields = iter.collect::<Vec<RecordField>>();
        match fields.len() {
            0 => {
                ((*sqlite3_api).result_null).unwrap()(ctx);
            }
            //            1 => {
            //                ((*sqlite3_api).result_blob).unwrap()(
            //                    ctx,
            //                    fields[0].data.as_ptr() as *const c_void,
            //                    fields[0].data.len() as c_int,
            //                    Some(SQLITE_TRANSIENT),
            //                );
            //            }
            _ => {
                let field_values = fields
                    .iter()
                    .map(|x| {
                        if x.has_subfields() {
                            x.subfield_iter()
                                .map(|y| y.utf8_data().to_owned())
                                .collect()
                        } else {
                            vec![x.utf8_data().to_owned()]
                        }
                    })
                    .collect::<Vec<Vec<String>>>();
                let serialized = serde_json::to_string(&field_values).expect("can't serialize");
                let l = serialized.as_bytes().len();
                let cstr = std::ffi::CString::new(serialized).expect("can't create cstring");
                ((*sqlite3_api).result_blob).unwrap()(
                    ctx,
                    cstr.as_ptr() as *const c_void,
                    l as c_int,
                    Some(SQLITE_TRANSIENT),
                );
            }
        }
    } else if i == field_types.len() {
        ((*sqlite3_api).result_int).unwrap()(ctx, record.header().record_length() as i32);
    } else if i == field_types.len() + 1 {
        let mut cur = Cursor::new(Vec::<u8>::new());
        if record.to_marc21(&mut cur).is_err() { return SQLITE_ERROR as c_int }
        let buflen = cur.get_ref().len();
        ((*sqlite3_api).result_blob).unwrap()(
            ctx,
            cur.get_ref().as_ptr() as *const c_void,
            buflen as c_int,
            Some(SQLITE_TRANSIENT),
        );
    } else if i == field_types.len() + 2 {
        let iter = MarcRecordFieldIter::new(&record, None);
        let types = iter.map(|field| field.field_type).collect::<Vec<usize>>();
        let serialized = serde_json::to_string(&types).expect("can't serialize");
        let l = serialized.as_bytes().len();
        let cstr = std::ffi::CString::new(serialized).expect("can't create cstring");
        ((*sqlite3_api).result_blob).unwrap()(
            ctx,
            cstr.as_ptr() as *const c_void,
            l as c_int,
            Some(SQLITE_TRANSIENT),
        );
    } else {
        unimplemented!();
    }
    return SQLITE_OK as i32;
}

unsafe extern "C" fn marcvtabRowid(cur: *mut sqlite3_vtab_cursor, pRowId: *mut i64) -> c_int {
    let pCur = cur as *mut marcvtab_cursor;
    *pRowId = (*pCur).iRowId as i64;
    return SQLITE_OK as i32;
}

unsafe extern "C" fn marcvtabEof(cur: *mut sqlite3_vtab_cursor) -> c_int {
    let pCur = cur as *mut marcvtab_cursor;
    if (*pCur).reader.as_ref().map(|x| x.is_eof()).unwrap_or(false) {
        return 1;
    }

    if (*pCur).iRowId >= 1000000 {
        return 1;
    } else {
        return 0;
    }
}

unsafe extern "C" fn marcvtabFilter(
    cur: *mut sqlite3_vtab_cursor,
    _idxNum: c_int,
    _idxStr: *const c_char,
    _argc: c_int,
    _argv: *mut *mut sqlite3_value,
) -> c_int {
    let pCur = cur as *mut marcvtab_cursor;
    (*pCur).iRowId = 1;
    if let Ok(_)=(*pCur).reader.as_mut().unwrap().advance() {
    SQLITE_OK as c_int
    } else {
        SQLITE_ERROR as c_int
    }
}

unsafe extern "C" fn marcvtabBestIndex(
    _tab: *mut sqlite3_vtab,
    pIdxInfo: *mut sqlite3_index_info,
) -> c_int {
    (*pIdxInfo).estimatedCost = 10.0;
    (*pIdxInfo).estimatedRows = 10;
    return SQLITE_OK as i32;
}

pub static marcvtabModule: sqlite3_module = sqlite3_module {
    iVersion: 1,
    //xCreate: None,
    xCreate: Some(marcvtabCreate),
    xConnect: Some(marcvtabConnect),
    xBestIndex: Some(marcvtabBestIndex),
    xDisconnect: Some(marcvtabDisconnect),
    xDestroy: Some(marcvtabDisconnect),
    xOpen: Some(marcvtabOpen),
    xClose: Some(marcvtabClose),
    xFilter: Some(marcvtabFilter),
    xNext: Some(marcvtabNext),
    xEof: Some(marcvtabEof),
    xColumn: Some(marcvtabColumn),
    xRowid: Some(marcvtabRowid),
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
    _pzErrMsg: *mut *mut c_char,
    pApi: *const sqlite3_api_routines,
) -> c_int {
    sqlite3_api = pApi;
    let s = std::ffi::CString::new("marcvtab").expect("Can't alloc string");
    let rc =
        (*sqlite3_api).create_module.unwrap()(db, s.as_ptr(), &marcvtabModule, std::ptr::null_mut());
    return rc as c_int;
}
