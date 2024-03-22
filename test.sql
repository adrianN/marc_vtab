.load '/home/adrian/marc_vtab/target/debug/libmarcvtab'
pragma module_list;
create virtual table if not exists myvtab using myvtab(authorities.mrc, 1,3, 65);
select * from myvtab order by entry_length limit 20;
select x65 from myvtab order by entry_length limit 20;
select count(*) from myvtab;
