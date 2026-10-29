pub mod ast;
pub mod lexer;
pub mod parser;
pub mod token;

pub use ast::*;
pub use lexer::{Lexer, LexerError};
pub use parser::{Parser, ParserError};
pub use token::{Span, Token, TokenKind};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lex_and_parse_table() {
        let source = r#"
            table Customer {
                caption = "Customer Master";
                tableType = Normal;
                dataPerCompany = true;

                id: UUID;
                code: String(20) {
                    notNull = true;
                    unique = true;
                };
                name: String(100);
                balance: Decimal(18, 2) {
                    computed = sum(CustomerLedgerEntry.amount where { customerId == id });
                };

                primaryKey(id);
                key(code);

                onInsert() {
                    if (this.code == null) {
                        this.code = "CUST-001";
                    }
                }
            }
        "#;

        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().expect("Lexing failed");
        let mut parser = Parser::new(tokens);
        let program = parser.parse_program().expect("Parsing failed");

        assert_eq!(program.items.len(), 1);
        match &program.items[0] {
            Item::Table(t) => {
                assert_eq!(t.name, "Customer");
                assert_eq!(t.fields.len(), 4);
                assert_eq!(t.primary_key, vec!["id"]);
                assert_eq!(t.triggers.len(), 1);
            }
            _ => panic!("Expected table"),
        }
    }

    #[test]
    fn test_parse_table_with_typescript_trigger() {
        let source = r#"
            table SecUser {
                username: String(50) { notNull = true; };
                passwordHash: String(255);

                onInsert(rec, ctx) {
                    if (!rec.username) throw new Error("Username required");
                    const salt = ctx.crypto.randomBytes(8).toHex();
                    rec.passwordHash = `$sha256$${salt}$${ctx.crypto.sha256(salt + rec.password)}`;
                    delete rec.password;
                }
            }
        "#;

        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().expect("Lexing failed");
        let mut parser = Parser::with_source(tokens, source.to_string());
        let program = parser.parse_program().expect("Parsing failed");

        assert_eq!(program.items.len(), 1);
        if let Item::Table(t) = &program.items[0] {
            assert_eq!(t.name, "SecUser");
            assert_eq!(t.triggers.len(), 1);
            let trg = &t.triggers[0];
            assert_eq!(trg.name, "onInsert");
            assert_eq!(trg.params, vec!["rec", "ctx"]);
            assert!(trg.body.raw_source.is_some());
            let script = trg.body.raw_source.as_ref().unwrap();
            assert!(script.contains("throw new Error"));
            assert!(script.contains("ctx.crypto.sha256"));
            assert!(script.contains("delete rec.password"));
        } else {
            panic!("Expected Table");
        }
    }

    #[test]
    fn test_event_keeps_template_and_regex_braces() {
        let source = r#"
            tablelogic NoteLogic for Note {
                onInsert(rec, ctx) {
                    let dt: Date = new Date();
                    const pattern = /}/;
                    const half = rec.qty / 2;
                    ctx.ui.message(`date ${dt}`, "info");
                }
            }
        "#;

        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().expect("Lexing failed");
        let mut parser = Parser::with_source(tokens, source.to_string());
        let program = parser.parse_program().expect("Parsing failed");

        let Item::TableLogic(logic) = &program.items[0] else {
            panic!("Expected TableLogic");
        };
        let script = logic.triggers[0].body.raw_source.as_deref().expect("event body");
        assert!(script.contains("/}/"), "{script}");
        assert!(script.contains("`date ${dt}`"), "{script}");
        assert!(script.contains("rec.qty / 2"), "{script}");
        assert!(script.contains("ctx.ui.message"), "{script}");
    }

    #[test]
    fn test_parse_permissionset_view_and_query() {
        let source = r#"
            permissionset SalesAdmin {
                caption = "Sales Administrator";
                permissions = {
                    Customer = RIMD;
                    SalesOrder = RIM where { amount <= 50000 };
                };
                execute = [OrderPostCodeunit, InvoiceService];
            }

            view CustomerSummaryView {
                caption = "Customer Summary View";
                materialized = false;
                select {
                    id,
                    name,
                    code
                }
                from Customer;
            }

            query TopCustomersQuery {
                caption = "Top Customers By Volume";
                elements {
                    dataitem(cust; Customer) {
                        column(custId; id) {}
                        column(custName; name) {}
                        dataitem(orders; SalesOrder) {
                            left join (orders.customerId == cust.id);
                            column(totalAmount; amount; sum) {}
                        }
                    }
                }
                groupBy(cust.id, cust.name);
                orderBy(totalAmount desc);
            }
        "#;

        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().expect("Lexing failed");
        let mut parser = Parser::new(tokens);
        let program = parser.parse_program().expect("Parsing failed");

        assert_eq!(program.items.len(), 3);

        match &program.items[0] {
            Item::PermissionSet(ps) => {
                assert_eq!(ps.name, "SalesAdmin");
                assert_eq!(ps.permissions.len(), 2);
                assert_eq!(ps.permissions[0].table_name, "Customer");
                assert_eq!(ps.permissions[0].actions.len(), 4); // R, I, M, D
                assert_eq!(ps.permissions[1].table_name, "SalesOrder");
                assert_eq!(ps.permissions[1].actions.len(), 3); // R, I, M
                assert!(ps.permissions[1].row_filter.is_some());
                assert_eq!(ps.execute, vec!["OrderPostCodeunit", "InvoiceService"]);
            }
            _ => panic!("Expected PermissionSet"),
        }

        match &program.items[1] {
            Item::View(v) => {
                assert_eq!(v.name, "CustomerSummaryView");
                assert_eq!(v.source_table, "Customer");
                assert_eq!(v.select_fields.len(), 3);
            }
            _ => panic!("Expected View"),
        }

        match &program.items[2] {
            Item::Query(q) => {
                assert_eq!(q.name, "TopCustomersQuery");
                assert_eq!(q.elements.len(), 1);
                let root_elem = &q.elements[0];
                assert_eq!(root_elem.name, "cust");
                assert_eq!(root_elem.table_name, "Customer");
                assert_eq!(root_elem.columns.len(), 2);
                assert_eq!(root_elem.data_items.len(), 1);
                assert_eq!(q.group_by.len(), 2);
                assert_eq!(q.order_by.len(), 1);
                assert_eq!(q.order_by[0].column, "totalAmount");
                assert_eq!(q.order_by[0].ascending, false);
            }
            _ => panic!("Expected Query"),
        }
    }

    #[test]
    fn test_parse_table_logic_and_extension() {
        let source = r#"
            tablelogic UserLogic for User {
                validate email(val) {
                    if (!val.includes("@")) {
                        throw new Error("Invalid email");
                    }
                }

                beforeInsert(rec, ctx) {
                    rec.createdAt = new Date().toISOString();
                }

                isSuperAdmin(rec) {
                    return rec.role === "Admin";
                }
            }

            tablelogicextension UserAuditLogic for User {
                afterModify(rec, oldRec, ctx) {
                    ctx.logAudit("User modified", rec.id);
                }
            }
        "#;

        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().expect("Lexing failed");
        let mut parser = Parser::with_source(tokens, source.to_string());
        let program = parser.parse_program().expect("Parsing failed");

        assert_eq!(program.items.len(), 2);

        match &program.items[0] {
            Item::TableLogic(tl) => {
                assert_eq!(tl.name, "UserLogic");
                assert_eq!(tl.target_table, "User");
                assert_eq!(tl.field_validations.len(), 1);
                assert_eq!(tl.field_validations[0].field_name, "email");
                assert_eq!(tl.field_validations[0].param_name, "val");
                assert_eq!(tl.triggers.len(), 1);
                assert_eq!(tl.triggers[0].name, "beforeInsert");
                assert_eq!(tl.methods.len(), 1);
                assert_eq!(tl.methods[0].name, "isSuperAdmin");
            }
            _ => panic!("Expected TableLogic"),
        }

        match &program.items[1] {
            Item::TableLogicExtension(tle) => {
                assert_eq!(tle.name, "UserAuditLogic");
                assert_eq!(tle.target_table, "User");
                assert_eq!(tle.triggers.len(), 1);
                assert_eq!(tle.triggers[0].name, "afterModify");
            }
            _ => panic!("Expected TableLogicExtension"),
        }
    }

    #[test]
    fn test_parse_page_report_custommetadata_and_async_codeunit() {
        let source = r#"
            page CustomerCard for Customer {
                pageType: Card;
                caption: "Customer Card";
                layout {
                    area(Content) {
                        group(General) {
                            field(code) { caption = "No."; };
                            field(name) { caption = "Customer Name"; };
                        }
                    }
                }
                actions {
                    action(Approve) {
                        caption: "Approve Customer";
                    }
                }
            }

            pageextension CustomerCardExt extends CustomerCard {
                layout {
                    field(loyalty_tier) { caption: "Loyalty Tier"; }
                }
            }

            report CustomerTrialBalance {
                caption: "Customer Trial Balance";
                dataset: Customer;
                columns {
                    column(id, id);
                    column(name, name);
                }
            }

            custommetadata TaxEngineSetting {
                provider_code: String(30) notNull;
                endpoint_url: String(250);
            }

            statemachine DocumentWorkflow for SalesHeader {
                initial: Draft;
                states: [Draft, PendingApproval, Released];
                transition submit from Draft to PendingApproval;
                transition release from PendingApproval to Released;
            }

            codeunit SalesOrderOrchestrator {
                @subscribeEvent(SalesPost, "onAfterPost")
                async function execute(id: UUID, mode: string | null): Promise<boolean> {
                    const rec = await models.SalesHeader.get(id);
                    return rec !== null;
                }
            }
        "#;

        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().expect("Lexing failed");
        let mut parser = Parser::with_source(tokens, source.to_string());
        let program = parser.parse_program().expect("Parsing failed");

        assert_eq!(program.items.len(), 6);
        assert!(matches!(&program.items[0], Item::Page(p) if p.name == "CustomerCard" && p.source_table.as_deref() == Some("Customer")));
        assert!(matches!(&program.items[1], Item::PageExtension(pe) if pe.name == "CustomerCardExt" && pe.target_page == "CustomerCard"));
        assert!(matches!(&program.items[2], Item::Report(r) if r.name == "CustomerTrialBalance" && r.source_table.as_deref() == Some("Customer")));
        assert!(matches!(&program.items[3], Item::CustomMetadata(cm) if cm.name == "TaxEngineSetting" && cm.fields.len() == 2));
        assert!(matches!(&program.items[4], Item::StateMachine(sm) if sm.name == "DocumentWorkflow" && sm.states.len() == 3 && sm.transitions.len() == 2));
        assert!(matches!(&program.items[5], Item::Codeunit(cu) if cu.name == "SalesOrderOrchestrator" && cu.methods.len() == 1));
    }

    #[test]
    fn test_parse_arbitrary_layout_page_and_extension_directives() {
        let source = r#"
            page SalesOrderWorkspace for SalesHeader {
                caption: "Sales Order Workspace";
                pageType = Workspace;

                layout {
                    flex(direction = "row", gap = 4) {
                        card("KPI Summary") {
                            field(totalAmount) { readonly; }
                            field(status);
                        }
                    }

                    split(ratio = "70:30") {
                        tabs {
                            tab("General") {
                                grid(cols = 3, gap = 2) {
                                    field(orderNo);
                                    field(customerId);
                                    field(orderDate);
                                }
                            }
                            tab("Lines") {
                                repeater SalesLines {
                                    field(itemNo);
                                    field(quantity);
                                    field(unitPrice);
                                }
                            }
                        }
                        accordion {
                            panel("FactBox") {
                                field(customerBalance);
                            }
                        }
                    }
                }
            }

            pageextension SuperLogisticsExt extends SalesOrderWorkspace {
                layout {
                    addAfter(orderDate) {
                        field(trackingNumber);
                        field(vesselName);
                    }
                    modify(customerId) {
                        label = "Client Account";
                        required;
                    }
                    wrap(customerBalance) {
                        flex(gap = 2) {
                            field(creditLimit);
                        }
                    }
                }
            }
        "#;

        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().expect("Lexing failed");
        let mut parser = Parser::with_source(tokens, source.to_string());
        let program = parser.parse_program().expect("Parsing failed");

        assert_eq!(program.items.len(), 2);
        match &program.items[0] {
            Item::Page(p) => {
                assert_eq!(p.name, "SalesOrderWorkspace");
                assert_eq!(p.controls.len(), 2); // flex and split
                assert_eq!(p.controls[0].kind, "flex");
                assert_eq!(p.controls[1].kind, "split");
            }
            _ => panic!("Expected Page"),
        }
        match &program.items[1] {
            Item::PageExtension(pe) => {
                assert_eq!(pe.name, "SuperLogisticsExt");
                assert_eq!(pe.target_page, "SalesOrderWorkspace");
                assert_eq!(pe.controls.len(), 3);
                assert_eq!(pe.controls[0].kind, "addAfter");
                assert_eq!(pe.controls[0].name, "orderDate");
                assert_eq!(pe.controls[1].kind, "modify");
                assert_eq!(pe.controls[1].name, "customerId");
                assert_eq!(pe.controls[2].kind, "wrap");
                assert_eq!(pe.controls[2].name, "customerBalance");
            }
            _ => panic!("Expected PageExtension"),
        }
    }

    fn parse(source: &str) -> Program {
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().expect("Lexing failed");
        let mut parser = Parser::with_source(tokens, source.to_string());
        parser.parse_program().expect("Parsing failed")
    }

    #[test]
    fn test_parse_page_v2_style_responsive_variables_and_action_areas() {
        let program = parse(
            r##"
            page SignIn for AuthConfiguration {
                pageType = StandardDialog;
                style { background = surface; radius = lg; }
                variables {
                    password: String(128) { caption = "Password"; };
                    rememberMe: Boolean = false;
                }
                layout {
                    area(Content) {
                        group(Credentials) {
                            field(password) {
                                extendedDatatype = Masked;
                                style { borderColor = "#1e40af"; borderWidth = 1; emphasis = Attention; icon = "lock"; }
                                responsive { md = 6; hideBelow = sm; }
                            }
                            spacer;
                        }
                    }
                }
                actions {
                    area(Processing) {
                        group(Session) {
                            caption = "Session";
                            action(SignIn) {
                                caption = "Sign In";
                                style { icon = "log-in"; }
                                onAction(rec, ctx) { ctx.ui.toast("ok"); }
                            }
                            separator;
                        }
                    }
                    area(Promoted) {
                        actionref(SignIn_Promoted; SignIn) {}
                    }
                }
            }
            "##,
        );
        let Item::Page(p) = &program.items[0] else { panic!("expected page") };
        assert_eq!(p.style.len(), 2);
        assert_eq!(p.variables.len(), 2);
        assert_eq!(p.variables[0].name, "password");
        assert_eq!(p.variables[0].field_type, FieldType::String(Some(128)));
        assert_eq!(p.variables[1].properties[0].name, "default");

        let field = &p.controls[0].children[0].children[0];
        assert_eq!(field.kind, "field");
        assert_eq!(field.style.len(), 4);
        assert_eq!(field.responsive.len(), 2);
        assert_eq!(p.controls[0].children[0].children[1].kind, "spacer");

        assert_eq!(p.actions.len(), 2);
        let processing = &p.actions[0];
        assert_eq!((processing.kind.as_str(), processing.name.as_str()), ("area", "Processing"));
        let group = &processing.children[0];
        assert_eq!(group.kind, "group");
        assert_eq!(group.children.len(), 2);
        assert_eq!(group.children[0].name, "SignIn");
        assert!(group.children[0].on_action.is_some());
        assert_eq!(group.children[0].style.len(), 1);
        assert_eq!(group.children[1].kind, "separator");
        let promoted = &p.actions[1].children[0];
        assert_eq!(promoted.kind, "actionref");
        assert_eq!(promoted.target.as_deref(), Some("SignIn"));
    }

    #[test]
    fn test_parse_page_extension_move_and_action_directives() {
        let program = parse(
            r##"
            pageextension CustomerCardCustomization extends CustomerCard {
                caption = "Client";
                layout {
                    moveAfter(name; creditLimit);
                    moveFirst(General) { balance; }
                    modify(name) { style { background = "#FFF7ED"; } }
                }
                actions {
                    addAfter(Post) { action(PostAndPrint) { caption = "Post and Print"; } }
                    modify(Post) { caption = "Post Invoice"; }
                    hide(Delete);
                }
            }
            "##,
        );
        let Item::PageExtension(pe) = &program.items[0] else { panic!("expected page extension") };
        assert_eq!(pe.properties[0].name, "caption");
        assert_eq!(pe.controls[0].kind, "moveAfter");
        assert_eq!(pe.controls[0].name, "name");
        assert_eq!(pe.controls[0].source_expr.as_deref(), Some("creditLimit"));
        assert_eq!(pe.controls[1].kind, "moveFirst");
        assert_eq!(pe.controls[1].children[0].name, "balance");
        assert_eq!(pe.controls[2].style.len(), 1);
        assert_eq!(pe.actions.len(), 3);
        assert_eq!((pe.actions[0].kind.as_str(), pe.actions[0].name.as_str()), ("addAfter", "Post"));
        assert_eq!(pe.actions[0].children[0].name, "PostAndPrint");
        assert_eq!(pe.actions[1].kind, "modify");
        assert_eq!((pe.actions[2].kind.as_str(), pe.actions[2].name.as_str()), ("hide", "Delete"));
    }

    #[test]
    fn test_syntax_symmetry_and_flexible_grammar() {
        let program = parse(
            r##"
            tablelogic VendorLogic for Vendor {
                validate(name) {
                    if (!val) throw new Error("Name required");
                }
                validate(email, customVal) {
                    if (!customVal.includes("@")) throw new Error("Invalid email");
                }
            }

            statemachineextension OrderWorkflowExt extends OrderWorkflow {
                states: [Shipped, Returned];
                state Delivered;
                transition ship from Ordered to Shipped;
            }

            permissionset FlexibleSecurity {
                permissions = {
                    Vendor = RIMD,
                    Order = RIMD;
                };
            }

            view OpenOrdersView {
                select { id, orderNo }
                from Order
                where { isClosed == false };
            }
            "##,
        );

        assert_eq!(program.items.len(), 4);
        let Item::TableLogic(tl) = &program.items[0] else { panic!("expected tablelogic") };
        assert_eq!(tl.field_validations.len(), 2);
        assert_eq!(tl.field_validations[0].field_name, "name");
        assert_eq!(tl.field_validations[1].field_name, "email");
        assert_eq!(tl.field_validations[1].param_name, "customVal");

        let Item::StateMachineExtension(sme) = &program.items[1] else { panic!("expected statemachineextension") };
        assert_eq!(sme.states.len(), 3);
        assert_eq!(sme.states[0].name, "Shipped");
        assert_eq!(sme.states[1].name, "Returned");
        assert_eq!(sme.states[2].name, "Delivered");

        let Item::PermissionSet(ps) = &program.items[2] else { panic!("expected permissionset") };
        assert_eq!(ps.permissions.len(), 2);

        let Item::View(v) = &program.items[3] else { panic!("expected view") };
        assert!(v.where_filter.is_some());
    }
}

