/**
 * BOS Core API Catalog & Automated Test Definitions
 * Covers all 52 Axum REST endpoints defined in crates/bosc-runtime/src/server.rs
 */

export const API_CATEGORIES = [
  { id: 'all', label: 'All Endpoints', icon: 'Sparkles' },
  { id: 'system', label: 'System & Health', icon: 'Activity' },
  { id: 'databases', label: 'Database Cluster', icon: 'Settings' },
  { id: 'metadata', label: 'Meta Catalog & DSL', icon: 'FileCode' },
  { id: 'packages', label: 'Packages (.bospkg)', icon: 'Layers' },
  { id: 'entities', label: 'Entities & CRUD', icon: 'Database' },
  { id: 'workflows', label: 'Workflows & States', icon: 'Sliders' },
  { id: 'query', label: 'Query & SIFT Views', icon: 'Search' },
  { id: 'events', label: 'Event Engine & Outbox', icon: 'Radio' },
  { id: 'codeunits', label: 'Codeunits & Tests', icon: 'Code2' },
  { id: 'wasm', label: 'Wasm & Sidecars', icon: 'Cpu' },
  { id: 'security', label: 'Security & RBAC', icon: 'Shield' }
];

export const PRESET_ENDPOINTS = [
  // 1. System & Health
  {
    id: 'health',
    category: 'system',
    method: 'GET',
    path: '/api/health',
    name: 'System Health Check',
    description: 'Verify BOS core Axum server liveness and service status.',
    sampleBody: ''
  },
  {
    id: 'detect-db',
    category: 'system',
    method: 'GET',
    path: '/api/detect-db',
    name: 'Auto-Detect Local Databases',
    description: 'Scans for running PostgreSQL, SQLite, MySQL, and Redis instances on host.',
    sampleBody: ''
  },
  {
    id: 'meta-system-status',
    category: 'system',
    method: 'GET',
    path: '/api/meta/system-status',
    name: 'Kernel Provisioning Status',
    description: 'Check active schema version, kernel version, and storage strategy.',
    sampleBody: ''
  },
  {
    id: 'meta-provision',
    category: 'system',
    method: 'POST',
    path: '/api/meta/provision',
    name: 'Provision System Kernel',
    description: 'Bootstrap system tables (_bos_sys_info, _bos_meta_objects, catalogs).',
    sampleBody: JSON.stringify({ storage_strategy: 'shared_rls' }, null, 2)
  },
  {
    id: 'storage-strategies',
    category: 'system',
    method: 'GET',
    path: '/api/system/storage-strategies',
    name: 'List Storage Strategies',
    description: 'Inspect multi-tenant topologies (SharedRLS, SchemaPerCompany, TablePerCompany, Partitioned).',
    sampleBody: ''
  },

  // 2. Database Cluster
  {
    id: 'list-databases',
    category: 'databases',
    method: 'GET',
    path: '/api/databases',
    name: 'List Cluster Databases',
    description: 'Inspect PostgreSQL database catalogs and active connection status.',
    sampleBody: ''
  },
  {
    id: 'test-connection',
    category: 'databases',
    method: 'POST',
    path: '/api/databases/test-connection',
    name: 'Test Connection URL',
    description: 'Ping target database URL and measure network latency.',
    sampleBody: JSON.stringify({
      url: ''
    }, null, 2)
  },
  {
    id: 'create-database',
    category: 'databases',
    method: 'POST',
    path: '/api/databases',
    name: 'Create Database Catalog',
    description: 'Provision a new PostgreSQL database with uuid-ossp and pgcrypto extensions.',
    sampleBody: JSON.stringify({
      name: 'bos_sandbox_db',
      init_extensions: true,
      provision_system: true,
      storage_strategy: 'shared_rls'
    }, null, 2)
  },
  {
    id: 'switch-database',
    category: 'databases',
    method: 'POST',
    path: '/api/databases/switch',
    name: 'Switch Active Database',
    description: 'Live-migrate runtime connection pool to target database with zero downtime.',
    sampleBody: JSON.stringify({ name: 'bos_dev' }, null, 2)
  },
  {
    id: 'get-current-database',
    category: 'databases',
    method: 'GET',
    path: '/api/databases/current',
    name: 'Get Current Active Database',
    description: 'Fetch details, schema, and connection configuration of current database.',
    sampleBody: ''
  },
  {
    id: 'inspect-database',
    category: 'databases',
    method: 'GET',
    path: '/api/databases/bos_dev',
    name: 'Inspect Database Details',
    description: 'Query table catalogs, storage size, and statistics of target database.',
    sampleBody: ''
  },
  {
    id: 'manage-database',
    category: 'databases',
    method: 'POST',
    path: '/api/databases/current/manage',
    name: 'Execute DDL Maintenance',
    description: 'Run administrative SQL statements (VACUUM, ANALYZE, REINDEX, TRUNCATE).',
    sampleBody: JSON.stringify({
      action: 'analyze',
      table_name: 'user'
    }, null, 2)
  },

  // 3. Meta Catalog & DSL
  {
    id: 'list-meta-objects',
    category: 'metadata',
    method: 'GET',
    path: '/api/meta/objects',
    name: 'List All Meta-Objects',
    description: 'Inspect tables, tablelogics, codeunits, views, and state machines in catalog.',
    sampleBody: ''
  },
  {
    id: 'get-meta-object',
    category: 'metadata',
    method: 'GET',
    path: '/api/meta/objects/User',
    name: 'Get Meta-Object Details',
    description: 'Retrieve definition IR, columns, and foreign keys for target meta-object.',
    sampleBody: ''
  },
  {
    id: 'ambient-dts',
    category: 'metadata',
    method: 'GET',
    path: '/api/meta/ambient.d.ts',
    name: 'Get Ambient TypeScript Definitions',
    description: 'Auto-generate global TypeScript typings for all registered database entities.',
    sampleBody: ''
  },
  {
    id: 'compile-script',
    category: 'metadata',
    method: 'POST',
    path: '/api/script/compile',
    name: 'Compile BOS-Script DSL',
    description: 'Transpile BOS-Script AST into PostgreSQL DDL and bytecode ModelIR.',
    sampleBody: JSON.stringify({
      script: `table Customer {
    code: String(20) { notNull = true; unique = true; };
    name: String(100);
    creditLimit: Decimal(18, 2);
    primaryKey(code);
}

tablelogic CustomerLogic for Customer {
    validate creditLimit(val) {
        if (val < 0) throw new Error("Credit limit cannot be negative");
    }
}`,
      package_name: 'bos.custom',
      apply: false
    }, null, 2)
  },
  {
    id: 'apply-script',
    category: 'metadata',
    method: 'POST',
    path: '/api/script/apply',
    name: 'Hot-Apply BOS-Script DDL',
    description: 'Atomically transpile and apply DDL directly to PostgreSQL with zero restart.',
    sampleBody: JSON.stringify({
      script: `table Contact {
    id: UUID { notNull = true; };
    name: String(100) { notNull = true; };
    email: String(100);
    primaryKey(id);
}`,
      package_name: 'bos.app'
    }, null, 2)
  },
  {
    id: 'apply-table-script',
    category: 'metadata',
    method: 'POST',
    path: '/api/meta/tables/Customer/script',
    name: 'Apply Table Extension Script',
    description: 'Non-invasively augment existing table with added columns or logic.',
    sampleBody: JSON.stringify({
      script: `tableextension CustomerLoyalty for Customer {
    loyaltyPoints: Integer { default = 0; };
}`,
      dry_run: false
    }, null, 2)
  },

  // 4. Packages (.bospkg)
  {
    id: 'list-packages',
    category: 'packages',
    method: 'GET',
    path: '/api/meta/packages',
    name: 'List Installed Packages',
    description: 'View installed packages, versions, checksums, and publisher metadata.',
    sampleBody: ''
  },
  {
    id: 'install-package',
    category: 'packages',
    method: 'POST',
    path: '/api/packages/install',
    name: 'Install Package Bundle',
    description: 'Install and activate verified .bospkg bundle with schema migrations.',
    sampleBody: JSON.stringify({
      manifest: {
        package_id: 'bos.demo',
        name: 'Demo Business Solution',
        version: '1.0.0',
        publisher: 'BOS Community'
      },
      objects: []
    }, null, 2)
  },

  // 5. Entities & CRUD
  {
    id: 'query-entities',
    category: 'entities',
    method: 'GET',
    path: '/api/entities/user?page=1&limit=25',
    name: 'Query Entities (Paginated)',
    description: 'Generic query with pagination headers (x-total-count, x-has-more, x-page).',
    sampleBody: ''
  },
  {
    id: 'entity-schema',
    category: 'entities',
    method: 'GET',
    path: '/api/entities/user/schema',
    name: 'Introspect Entity Schema',
    description: 'Query column types, constraints, default values, and relations.',
    sampleBody: ''
  },
  {
    id: 'create-entity',
    category: 'entities',
    method: 'POST',
    path: '/api/entities/user',
    name: 'Create Entity Record',
    description: 'Insert new entity record with OnBeforeInsert and OnAfterInsert triggers.',
    sampleBody: JSON.stringify({
      username: 'test_dev_' + Math.floor(Math.random() * 10000),
      email: 'test_dev@example.com',
      full_name: 'Developer Test Account',
      is_active: true
    }, null, 2)
  },
  {
    id: 'batch-insert',
    category: 'entities',
    method: 'POST',
    path: '/api/entities/user/batch',
    name: 'Batch Vector Insert',
    description: 'Bulk insert multiple records in a single vectorized multi-row SQL insert.',
    sampleBody: JSON.stringify([
      { username: 'batch_user_1', email: 'batch1@bos.internal', full_name: 'Batch 1' },
      { username: 'batch_user_2', email: 'batch2@bos.internal', full_name: 'Batch 2' }
    ], null, 2)
  },

  // 6. Workflows & States
  {
    id: 'state-transition',
    category: 'workflows',
    method: 'POST',
    path: '/api/entities/purchase_order/a0000000-0000-4000-8000-000000000001/transition',
    name: 'Execute State Transition',
    description: 'Atomically commit state change with guard checks and history logging.',
    sampleBody: JSON.stringify({
      transition: 'Submit',
      status_column: 'status',
      comments: 'Approved by department manager'
    }, null, 2)
  },
  {
    id: 'state-history',
    category: 'workflows',
    method: 'GET',
    path: '/api/entities/purchase_order/a0000000-0000-4000-8000-000000000001/state-history',
    name: 'Get State Audit History',
    description: 'Query immutable state transition trail with timestamps and user IDs.',
    sampleBody: ''
  },
  {
    id: 'privileged-override',
    category: 'workflows',
    method: 'POST',
    path: '/api/entities/customer_ledger_entry/a0000000-0000-4000-8000-000000000001/privileged-override',
    name: 'Privileged Statutory Override',
    description: 'Modify immutable ledger entry with mandatory ticket and audit logging.',
    sampleBody: JSON.stringify({
      column: 'description',
      new_value: 'Corrected via Privileged Audit Override',
      reason: 'Customer PO reference correction requested by controller',
      ticket: 'FIN-JIRA-402',
      authorizer: 'CFO'
    }, null, 2)
  },

  // 7. Query & Views
  {
    id: 'graphql-query',
    category: 'query',
    method: 'POST',
    path: '/api/query',
    name: 'Execute SIFT Pushdown Query',
    description: 'Execute query with wildcard ILIKE, SIFT FlowFields, sorting, and joins.',
    sampleBody: JSON.stringify({
      entity: 'user',
      selects: ['username', 'email', 'full_name'],
      wheres: [['is_active', '=', true]],
      limit: 10
    }, null, 2)
  },
  {
    id: 'get-view',
    category: 'query',
    method: 'GET',
    path: '/api/views/CustomerSummaryView',
    name: 'Get View Metadata',
    description: 'Inspect relational projection view definition and source tables.',
    sampleBody: ''
  },
  {
    id: 'refresh-view',
    category: 'query',
    method: 'POST',
    path: '/api/views/CustomerSummaryView/refresh',
    name: 'Refresh Materialized View',
    description: 'Execute concurrent refresh on PostgreSQL materialized analytical view.',
    sampleBody: JSON.stringify({ concurrently: true }, null, 2)
  },

  // 8. Event Engine & Outbox
  {
    id: 'list-subscribers',
    category: 'events',
    method: 'GET',
    path: '/api/events/subscribers',
    name: 'List Event Subscribers',
    description: 'Inspect registered decoupled event handlers and priority chains.',
    sampleBody: ''
  },
  {
    id: 'publish-event',
    category: 'events',
    method: 'POST',
    path: '/api/events/publish',
    name: 'Publish Synchronous Event',
    description: 'Fire in-memory domain event dispatch through QuickJS handler pipelines.',
    sampleBody: JSON.stringify({
      publisher: 'SalesOrder',
      event_name: 'OnAfterRelease',
      data: { orderNo: 'SO-10001', totalAmount: 4500.00 }
    }, null, 2)
  },
  {
    id: 'list-outbox',
    category: 'events',
    method: 'GET',
    path: '/api/events/outbox?status=pending',
    name: 'List Transactional Outbox',
    description: 'Inspect pending transactional events awaiting background dispatch.',
    sampleBody: ''
  },
  {
    id: 'process-outbox',
    category: 'events',
    method: 'POST',
    path: '/api/events/outbox/process',
    name: 'Process Outbox Queue',
    description: 'Drain pending outbox events using SELECT FOR UPDATE SKIP LOCKED.',
    sampleBody: JSON.stringify({ batch_size: 50 }, null, 2)
  },

  // 9. Codeunits & Tests
  {
    id: 'execute-codeunit',
    category: 'codeunits',
    method: 'POST',
    path: '/api/codeunits/AuthManager/run',
    name: 'Execute Codeunit Procedure',
    description: 'Execute procedure in sandboxed QuickJS VM with SessionContext and prelude.',
    sampleBody: JSON.stringify({
      method: 'ValidateUserRegistration',
      params: { username: 'testuser', email: 'test@example.com' }
    }, null, 2)
  },
  {
    id: 'run-tests',
    category: 'codeunits',
    method: 'POST',
    path: '/api/tests/run',
    name: 'Run Unit Test Suite',
    description: 'Execute @test methods inside Codeunits with automated transaction rollback.',
    sampleBody: JSON.stringify({ filter: '' }, null, 2)
  },

  // 10. Wasm & Sidecars
  {
    id: 'list-wasm',
    category: 'wasm',
    method: 'GET',
    path: '/api/wasm/modules',
    name: 'List Loaded Wasm Modules',
    description: 'Inspect high-performance compiled WebAssembly extensions.',
    sampleBody: ''
  },
  {
    id: 'list-sidecars',
    category: 'wasm',
    method: 'GET',
    path: '/api/sidecars',
    name: 'List Microservice Sidecars',
    description: 'View registered loopback sidecars and service discovery endpoints.',
    sampleBody: ''
  },

  // 11. Security & RBAC
  {
    id: 'check-permission',
    category: 'security',
    method: 'POST',
    path: '/api/permissions/check',
    name: 'Check User Permission (RIMD)',
    description: 'Verify table Read, Insert, Modify, Delete rights under active security roles.',
    sampleBody: JSON.stringify({
      entity: 'user',
      action: 'Read'
    }, null, 2)
  },
  {
    id: 'list-tenants',
    category: 'security',
    method: 'GET',
    path: '/api/tenants',
    name: 'List Tenants',
    description: 'Inspect multi-tenant isolation registry and tenant codes.',
    sampleBody: ''
  },
  {
    id: 'list-companies',
    category: 'security',
    method: 'GET',
    path: '/api/companies',
    name: 'List Companies',
    description: 'View registered company partitions and active ledger status.',
    sampleBody: ''
  }
];

/**
 * Standard Automated Integration Suite Steps
 */
export const AUTOMATED_FLOW_STEPS = [
  { step: 1, name: 'System Liveness Health Check', method: 'GET', path: '/api/health', body: null },
  { step: 2, name: 'Database Provider Discovery', method: 'GET', path: '/api/detect-db', body: null },
  { step: 3, name: 'List Cluster Database Catalogs', method: 'GET', path: '/api/databases', body: null },
  { step: 4, name: 'Test Target Connection Latency', method: 'POST', path: '/api/databases/test-connection', body: { url: '' } },
  { step: 5, name: 'Kernel Metadata Provisioning Status', method: 'GET', path: '/api/meta/system-status', body: null },
  { step: 6, name: 'Multi-Tenant Storage Topologies', method: 'GET', path: '/api/system/storage-strategies', body: null },
  { step: 7, name: 'List Core Meta-Objects Catalog', method: 'GET', path: '/api/meta/objects', body: null },
  { step: 8, name: 'Query Installed Package Bundles', method: 'GET', path: '/api/meta/packages', body: null },
  { step: 9, name: 'Introspect User Entity Schema', method: 'GET', path: '/api/entities/user/schema', body: null },
  { step: 10, name: 'Query User Master Records', method: 'GET', path: '/api/entities/user?page=1&limit=5', body: null },
  { step: 11, name: 'Inspect Security Tenants', method: 'GET', path: '/api/tenants', body: null },
  { step: 12, name: 'Inspect Security Companies', method: 'GET', path: '/api/companies', body: null },
  { step: 13, name: 'Check Table Read Permission', method: 'POST', path: '/api/permissions/check', body: { entity: 'user', action: 'Read' } },
  { step: 14, name: 'Inspect Event Engine Subscribers', method: 'GET', path: '/api/events/subscribers', body: null },
  { step: 15, name: 'Inspect Outbox Message Queue', method: 'GET', path: '/api/events/outbox', body: null }
];
