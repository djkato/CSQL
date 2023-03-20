#CSQL (CSV TO SQL)
An assistant tool to easily map a CSV spreadsheets collumns to a database tables' fields. Great for mass uploading products from a manufacturer provided catalog to an e-shop.


####What does it do?
- [x] Checks if all rows in a spreadsheet collumn bound to a tables field abide by the fields' rules (variable type and length), shows errors and allows to fix them easily.
For non-technical people: makes sure the spreadsheet follows some database rules and doesn't break anything :)
- [ ] Check if the database already has such an entry, eg. product with the same name exists already, and allows to updating the data to the data present in the spreadsheet.
- [ ] Input Sanitisation, eg. Simple mass text update for capitalization, Spell checking
- [ ] Create a CSV from a given Database
- [ ] Support for all major database types
- [ ] Common e-commerce platform prefabs (Opencart, Prestashop, Saleor based, Strapi based..)
- [ ] Simple Database backups and undo database updates