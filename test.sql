.load '/home/adrian/marc_vtab/target/debug/libmarcvtab'
pragma module_list;
create virtual table if not exists myvtab using myvtab(authorities.mrc, 1,3);
select * from myvtab limit 20;
select count(*) from myvtab;
