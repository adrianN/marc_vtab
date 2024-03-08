.load '/home/adrian/marc_vtab/target/debug/libmarcvtab'
pragma module_list;
create virtual table if not exists myvtab using myvtab(5);
select * from myvtab;
