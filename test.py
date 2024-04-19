import sqlite3

def replace_non_printable(s):
    return s if s.isprintable() else repr(s)

def print_table(result):
  for row in result:
    print(" | ".join((replace_non_printable(x.decode('utf-8', errors='replace') if isinstance(x,bytes) else str(x)) for x in row)))

def run_and_print(query):
    global c
    print(query)
    c.execute(query)
    print_table(c.fetchall())



con = sqlite3.connect(":memory:")
con.enable_load_extension(True)
#con.load_extension('/home/adrian/marc_vtab/target/debug/libmarcvtab')
con.load_extension('/home/adrian/marc_vtab/target/release/libmarcvtab')

global c
c = con.cursor()
c.execute(
"create virtual table authorities using marcvtab(file=authorities.mrc, fields='1,3,5,8,35,65,100,150');")
c.execute(
"create virtual table all_dnb using marcvtab(file=all.mrc, fields='1,3,5,8,35,65,100,240');")

run_and_print(" select x35, x150 from authorities order by entry_length asc limit 1; ")
run_and_print(" select x65 from authorities order by entry_length desc limit 1; ")
run_and_print("select x35,x150, field_types from authorities where cast(x35 as varchar) like '%4275004%' order by entry_length desc limit 2")
run_and_print("select x35,x150 from authorities where exists ( select * from json_each(cast(x35 as varchar)) where json_each.value like '%4275004%')")
run_and_print("select x35, x100, x240 from all_dnb where cast(full_record as varchar) like '%Serapionsbrüder%' ")
run_and_print("select count(*) from authorities as auth, all_dnb as dnb where cast(auth.full_record as varchar) like '%E. T. A.%' and cast(dnb.full_record as varchar) like '%E. T. A.%'")
