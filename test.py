import sqlite3

def print_table(result):
  for row in result:
    print(" | ".join((x.decode('utf-8', errors='replace') if isinstance(x,bytes) else str(x) for x in row)))

con = sqlite3.connect(":memory")
con.enable_load_extension(True)
con.load_extension('/home/adrian/marc_vtab/target/debug/libmarcvtab')

c = con.cursor()
c.execute(
"create virtual table if not exists myvtab using myvtab(file=authorities.mrc, fields='1,3,65');")

c.execute(" select * from myvtab order by entry_length desc limit 20; ")
print_table(c.fetchall())
c.execute(" select x65 from myvtab order by entry_length desc limit 20; ")
print_table(c.fetchall())
