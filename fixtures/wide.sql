-- A synthetic 100-table schema: layout readability has to be verified at this
-- size, and no schema we could redistribute is both large enough and shaped
-- like a real application (see fixtures/README.md).
--
-- It is modelled on a multi-tenant SaaS, so it has the structure that makes
-- layout hard: a handful of hubs almost everything points at (`tenants`,
-- `users`), long chains (tenants -> workspaces -> projects -> documents ->
-- comments -> reactions), join tables, several namespaces, and a set of
-- reference tables with no relationships at all.
--
-- Written like a pg_dump: tables first, then the constraints and indexes.

CREATE TABLE public.tenants (
    id         bigint      PRIMARY KEY,
    name text NOT NULL,
    slug text NOT NULL UNIQUE,
    plan_id bigint,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE public.users (
    id         bigint      PRIMARY KEY,
    email text NOT NULL UNIQUE,
    full_name text,
    tenant_id bigint NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE public.teams (
    id         bigint      PRIMARY KEY,
    tenant_id bigint NOT NULL,
    name text NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE public.team_members (
    id         bigint      PRIMARY KEY,
    team_id bigint NOT NULL,
    user_id bigint NOT NULL,
    role_id bigint,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE public.roles (
    id         bigint      PRIMARY KEY,
    tenant_id bigint,
    name text NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE public.permissions (
    id         bigint      PRIMARY KEY,
    code text NOT NULL UNIQUE,
    description text,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE public.role_permissions (
    id         bigint      PRIMARY KEY,
    role_id bigint NOT NULL,
    permission_id bigint NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE public.invitations (
    id         bigint      PRIMARY KEY,
    tenant_id bigint NOT NULL,
    email text NOT NULL,
    invited_by bigint,
    role_id bigint,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE public.user_profiles (
    id         bigint      PRIMARY KEY,
    user_id bigint NOT NULL UNIQUE,
    bio text,
    country_id bigint,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE public.user_settings (
    id         bigint      PRIMARY KEY,
    user_id bigint NOT NULL UNIQUE,
    locale_id bigint,
    timezone_id bigint,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE public.sessions (
    id         bigint      PRIMARY KEY,
    user_id bigint NOT NULL,
    token text NOT NULL,
    expires_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE public.api_keys (
    id         bigint      PRIMARY KEY,
    tenant_id bigint NOT NULL,
    created_by bigint,
    label text,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE public.api_key_scopes (
    id         bigint      PRIMARY KEY,
    api_key_id bigint NOT NULL,
    permission_id bigint NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE public.oauth_providers (
    id         bigint      PRIMARY KEY,
    name text NOT NULL UNIQUE,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE public.oauth_identities (
    id         bigint      PRIMARY KEY,
    user_id bigint NOT NULL,
    provider_id bigint NOT NULL,
    external_id text,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE billing.plans (
    id         bigint      PRIMARY KEY,
    name text NOT NULL,
    monthly_cents integer NOT NULL,
    currency_id bigint,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE billing.plan_features (
    id         bigint      PRIMARY KEY,
    plan_id bigint NOT NULL,
    feature_id bigint NOT NULL,
    limit_value integer,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE billing.features (
    id         bigint      PRIMARY KEY,
    code text NOT NULL UNIQUE,
    name text,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE billing.subscriptions (
    id         bigint      PRIMARY KEY,
    tenant_id bigint NOT NULL,
    plan_id bigint NOT NULL,
    status text,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE billing.subscription_items (
    id         bigint      PRIMARY KEY,
    subscription_id bigint NOT NULL,
    feature_id bigint,
    quantity integer,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE billing.invoices (
    id         bigint      PRIMARY KEY,
    subscription_id bigint NOT NULL,
    number text NOT NULL UNIQUE,
    total_cents integer,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE billing.invoice_lines (
    id         bigint      PRIMARY KEY,
    invoice_id bigint NOT NULL,
    description text,
    amount_cents integer,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE billing.payments (
    id         bigint      PRIMARY KEY,
    invoice_id bigint NOT NULL,
    method_id bigint,
    amount_cents integer,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE billing.payment_methods (
    id         bigint      PRIMARY KEY,
    tenant_id bigint NOT NULL,
    brand text,
    last4 text,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE billing.refunds (
    id         bigint      PRIMARY KEY,
    payment_id bigint NOT NULL,
    amount_cents integer,
    reason text,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE billing.coupons (
    id         bigint      PRIMARY KEY,
    code text NOT NULL UNIQUE,
    percent_off integer,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE billing.coupon_redemptions (
    id         bigint      PRIMARY KEY,
    coupon_id bigint NOT NULL,
    subscription_id bigint NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE billing.credit_notes (
    id         bigint      PRIMARY KEY,
    invoice_id bigint NOT NULL,
    amount_cents integer,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE billing.tax_rates (
    id         bigint      PRIMARY KEY,
    country_id bigint,
    percent numeric(5,2),
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE billing.invoice_taxes (
    id         bigint      PRIMARY KEY,
    invoice_id bigint NOT NULL,
    tax_rate_id bigint NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE billing.usage_records (
    id         bigint      PRIMARY KEY,
    subscription_item_id bigint NOT NULL,
    quantity integer,
    recorded_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE public.workspaces (
    id         bigint      PRIMARY KEY,
    tenant_id bigint NOT NULL,
    name text NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE public.projects (
    id         bigint      PRIMARY KEY,
    workspace_id bigint NOT NULL,
    name text NOT NULL,
    owner_id bigint,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE public.project_members (
    id         bigint      PRIMARY KEY,
    project_id bigint NOT NULL,
    user_id bigint NOT NULL,
    role_id bigint,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE public.documents (
    id         bigint      PRIMARY KEY,
    project_id bigint NOT NULL,
    title text NOT NULL,
    author_id bigint,
    folder_id bigint,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE public.folders (
    id         bigint      PRIMARY KEY,
    project_id bigint NOT NULL,
    name text NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE public.document_revisions (
    id         bigint      PRIMARY KEY,
    document_id bigint NOT NULL,
    body text,
    author_id bigint,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE public.comments (
    id         bigint      PRIMARY KEY,
    document_id bigint NOT NULL,
    author_id bigint,
    body text,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE public.comment_reactions (
    id         bigint      PRIMARY KEY,
    comment_id bigint NOT NULL,
    user_id bigint NOT NULL,
    emoji text,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE public.attachments (
    id         bigint      PRIMARY KEY,
    document_id bigint,
    comment_id bigint,
    blob_id bigint NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE public.blobs (
    id         bigint      PRIMARY KEY,
    checksum text NOT NULL UNIQUE,
    bytes bigint,
    storage_id bigint,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE public.storage_backends (
    id         bigint      PRIMARY KEY,
    name text NOT NULL,
    region text,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE public.tags (
    id         bigint      PRIMARY KEY,
    tenant_id bigint NOT NULL,
    name text NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE public.taggings (
    id         bigint      PRIMARY KEY,
    tag_id bigint NOT NULL,
    document_id bigint NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE public.document_shares (
    id         bigint      PRIMARY KEY,
    document_id bigint NOT NULL,
    shared_with bigint,
    token text,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE public.templates (
    id         bigint      PRIMARY KEY,
    tenant_id bigint,
    name text NOT NULL,
    body text,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE public.favorites (
    id         bigint      PRIMARY KEY,
    user_id bigint NOT NULL,
    document_id bigint NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE public.boards (
    id         bigint      PRIMARY KEY,
    project_id bigint NOT NULL,
    name text NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE public.board_columns (
    id         bigint      PRIMARY KEY,
    board_id bigint NOT NULL,
    name text NOT NULL,
    position integer,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE public.cards (
    id         bigint      PRIMARY KEY,
    column_id bigint NOT NULL,
    title text NOT NULL,
    assignee_id bigint,
    document_id bigint,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE public.card_labels (
    id         bigint      PRIMARY KEY,
    card_id bigint NOT NULL,
    label_id bigint NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE public.labels (
    id         bigint      PRIMARY KEY,
    project_id bigint NOT NULL,
    name text NOT NULL,
    color text,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE public.checklists (
    id         bigint      PRIMARY KEY,
    card_id bigint NOT NULL,
    title text,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE public.checklist_items (
    id         bigint      PRIMARY KEY,
    checklist_id bigint NOT NULL,
    body text,
    done boolean,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE public.card_watchers (
    id         bigint      PRIMARY KEY,
    card_id bigint NOT NULL,
    user_id bigint NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE public.time_entries (
    id         bigint      PRIMARY KEY,
    card_id bigint NOT NULL,
    user_id bigint NOT NULL,
    minutes integer,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE public.milestones (
    id         bigint      PRIMARY KEY,
    project_id bigint NOT NULL,
    name text,
    due_on date,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE public.card_milestones (
    id         bigint      PRIMARY KEY,
    card_id bigint NOT NULL,
    milestone_id bigint NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE public.automations (
    id         bigint      PRIMARY KEY,
    board_id bigint NOT NULL,
    trigger text,
    action text,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE public.channels (
    id         bigint      PRIMARY KEY,
    workspace_id bigint NOT NULL,
    name text NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE public.channel_members (
    id         bigint      PRIMARY KEY,
    channel_id bigint NOT NULL,
    user_id bigint NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE public.messages (
    id         bigint      PRIMARY KEY,
    channel_id bigint NOT NULL,
    author_id bigint,
    body text,
    thread_id bigint,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE public.threads (
    id         bigint      PRIMARY KEY,
    channel_id bigint NOT NULL,
    title text,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE public.message_reactions (
    id         bigint      PRIMARY KEY,
    message_id bigint NOT NULL,
    user_id bigint NOT NULL,
    emoji text,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE public.mentions (
    id         bigint      PRIMARY KEY,
    message_id bigint NOT NULL,
    user_id bigint NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE public.read_receipts (
    id         bigint      PRIMARY KEY,
    message_id bigint NOT NULL,
    user_id bigint NOT NULL,
    read_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE public.notifications (
    id         bigint      PRIMARY KEY,
    user_id bigint NOT NULL,
    kind text,
    payload text,
    channel_id bigint,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE public.notification_preferences (
    id         bigint      PRIMARY KEY,
    user_id bigint NOT NULL,
    kind text,
    enabled boolean,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE public.email_deliveries (
    id         bigint      PRIMARY KEY,
    user_id bigint,
    template_id bigint,
    status text,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE public.email_templates (
    id         bigint      PRIMARY KEY,
    code text NOT NULL UNIQUE,
    subject text,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE public.integrations (
    id         bigint      PRIMARY KEY,
    tenant_id bigint NOT NULL,
    provider text NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE public.integration_configs (
    id         bigint      PRIMARY KEY,
    integration_id bigint NOT NULL,
    key text,
    value text,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE public.webhooks (
    id         bigint      PRIMARY KEY,
    tenant_id bigint NOT NULL,
    url text NOT NULL,
    integration_id bigint,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE public.webhook_deliveries (
    id         bigint      PRIMARY KEY,
    webhook_id bigint NOT NULL,
    status_code integer,
    body text,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE public.webhook_events (
    id         bigint      PRIMARY KEY,
    webhook_id bigint NOT NULL,
    event_type_id bigint NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE public.event_types (
    id         bigint      PRIMARY KEY,
    code text NOT NULL UNIQUE,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE public.imports (
    id         bigint      PRIMARY KEY,
    tenant_id bigint NOT NULL,
    source text,
    status text,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE public.import_rows (
    id         bigint      PRIMARY KEY,
    import_id bigint NOT NULL,
    line integer,
    error text,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE public.exports (
    id         bigint      PRIMARY KEY,
    tenant_id bigint NOT NULL,
    requested_by bigint,
    blob_id bigint,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE audit.audit_logs (
    id         bigint      PRIMARY KEY,
    tenant_id bigint,
    actor_id bigint,
    action text,
    event_type_id bigint,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE audit.audit_changes (
    id         bigint      PRIMARY KEY,
    audit_log_id bigint NOT NULL,
    field text,
    before text,
    after text,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE audit.login_attempts (
    id         bigint      PRIMARY KEY,
    user_id bigint,
    ip inet,
    succeeded boolean,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE audit.security_alerts (
    id         bigint      PRIMARY KEY,
    tenant_id bigint,
    severity text,
    audit_log_id bigint,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE ops.jobs (
    id         bigint      PRIMARY KEY,
    queue text NOT NULL,
    payload text,
    status text,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE ops.job_attempts (
    id         bigint      PRIMARY KEY,
    job_id bigint NOT NULL,
    attempt integer,
    error text,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE ops.schedules (
    id         bigint      PRIMARY KEY,
    job_id bigint,
    cron text,
    enabled boolean,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE ops.feature_flags (
    id         bigint      PRIMARY KEY,
    tenant_id bigint,
    feature_id bigint,
    enabled boolean,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE ops.rate_limits (
    id         bigint      PRIMARY KEY,
    api_key_id bigint,
    window_seconds integer,
    max_requests integer,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE ops.service_incidents (
    id         bigint      PRIMARY KEY,
    title text NOT NULL,
    started_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE ops.incident_updates (
    id         bigint      PRIMARY KEY,
    incident_id bigint NOT NULL,
    body text,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE ops.metrics_daily (
    id         bigint      PRIMARY KEY,
    tenant_id bigint NOT NULL,
    day date,
    documents integer,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE ops.search_indexes (
    id         bigint      PRIMARY KEY,
    document_id bigint NOT NULL,
    vector text,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE ops.cache_entries (
    id         bigint      PRIMARY KEY,
    key text NOT NULL UNIQUE,
    value text,
    expires_at timestamptz,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE ref.countries (
    id         bigint      PRIMARY KEY,
    iso2 text NOT NULL UNIQUE,
    name text NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE ref.currencies (
    id         bigint      PRIMARY KEY,
    code text NOT NULL UNIQUE,
    symbol text,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE ref.locales (
    id         bigint      PRIMARY KEY,
    code text NOT NULL UNIQUE,
    name text,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE ref.timezones (
    id         bigint      PRIMARY KEY,
    name text NOT NULL UNIQUE,
    utc_offset integer,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE ref.holidays (
    id         bigint      PRIMARY KEY,
    country_id bigint NOT NULL,
    day date,
    name text,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE ref.industries (
    id         bigint      PRIMARY KEY,
    name text NOT NULL UNIQUE,
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE ref.company_sizes (
    id         bigint      PRIMARY KEY,
    label text NOT NULL UNIQUE,
    min_seats integer,
    created_at timestamptz NOT NULL DEFAULT now()
);


--
-- Foreign keys.
--

ALTER TABLE ONLY public.tenants
    ADD CONSTRAINT tenants_plan_id_fkey
    FOREIGN KEY (plan_id) REFERENCES billing.plans (id);

ALTER TABLE ONLY public.users
    ADD CONSTRAINT users_tenant_id_fkey
    FOREIGN KEY (tenant_id) REFERENCES public.tenants (id);

ALTER TABLE ONLY public.teams
    ADD CONSTRAINT teams_tenant_id_fkey
    FOREIGN KEY (tenant_id) REFERENCES public.tenants (id);

ALTER TABLE ONLY public.team_members
    ADD CONSTRAINT team_members_team_id_fkey
    FOREIGN KEY (team_id) REFERENCES public.teams (id);

ALTER TABLE ONLY public.team_members
    ADD CONSTRAINT team_members_user_id_fkey
    FOREIGN KEY (user_id) REFERENCES public.users (id);

ALTER TABLE ONLY public.team_members
    ADD CONSTRAINT team_members_role_id_fkey
    FOREIGN KEY (role_id) REFERENCES public.roles (id);

ALTER TABLE ONLY public.roles
    ADD CONSTRAINT roles_tenant_id_fkey
    FOREIGN KEY (tenant_id) REFERENCES public.tenants (id);

ALTER TABLE ONLY public.role_permissions
    ADD CONSTRAINT role_permissions_role_id_fkey
    FOREIGN KEY (role_id) REFERENCES public.roles (id);

ALTER TABLE ONLY public.role_permissions
    ADD CONSTRAINT role_permissions_permission_id_fkey
    FOREIGN KEY (permission_id) REFERENCES public.permissions (id);

ALTER TABLE ONLY public.invitations
    ADD CONSTRAINT invitations_tenant_id_fkey
    FOREIGN KEY (tenant_id) REFERENCES public.tenants (id);

ALTER TABLE ONLY public.invitations
    ADD CONSTRAINT invitations_invited_by_fkey
    FOREIGN KEY (invited_by) REFERENCES public.users (id);

ALTER TABLE ONLY public.invitations
    ADD CONSTRAINT invitations_role_id_fkey
    FOREIGN KEY (role_id) REFERENCES public.roles (id);

ALTER TABLE ONLY public.user_profiles
    ADD CONSTRAINT user_profiles_user_id_fkey
    FOREIGN KEY (user_id) REFERENCES public.users (id);

ALTER TABLE ONLY public.user_profiles
    ADD CONSTRAINT user_profiles_country_id_fkey
    FOREIGN KEY (country_id) REFERENCES ref.countries (id);

ALTER TABLE ONLY public.user_settings
    ADD CONSTRAINT user_settings_user_id_fkey
    FOREIGN KEY (user_id) REFERENCES public.users (id);

ALTER TABLE ONLY public.user_settings
    ADD CONSTRAINT user_settings_locale_id_fkey
    FOREIGN KEY (locale_id) REFERENCES ref.locales (id);

ALTER TABLE ONLY public.user_settings
    ADD CONSTRAINT user_settings_timezone_id_fkey
    FOREIGN KEY (timezone_id) REFERENCES ref.timezones (id);

ALTER TABLE ONLY public.sessions
    ADD CONSTRAINT sessions_user_id_fkey
    FOREIGN KEY (user_id) REFERENCES public.users (id);

ALTER TABLE ONLY public.api_keys
    ADD CONSTRAINT api_keys_tenant_id_fkey
    FOREIGN KEY (tenant_id) REFERENCES public.tenants (id);

ALTER TABLE ONLY public.api_keys
    ADD CONSTRAINT api_keys_created_by_fkey
    FOREIGN KEY (created_by) REFERENCES public.users (id);

ALTER TABLE ONLY public.api_key_scopes
    ADD CONSTRAINT api_key_scopes_api_key_id_fkey
    FOREIGN KEY (api_key_id) REFERENCES public.api_keys (id);

ALTER TABLE ONLY public.api_key_scopes
    ADD CONSTRAINT api_key_scopes_permission_id_fkey
    FOREIGN KEY (permission_id) REFERENCES public.permissions (id);

ALTER TABLE ONLY public.oauth_identities
    ADD CONSTRAINT oauth_identities_user_id_fkey
    FOREIGN KEY (user_id) REFERENCES public.users (id);

ALTER TABLE ONLY public.oauth_identities
    ADD CONSTRAINT oauth_identities_provider_id_fkey
    FOREIGN KEY (provider_id) REFERENCES public.oauth_providers (id);

ALTER TABLE ONLY billing.plans
    ADD CONSTRAINT plans_currency_id_fkey
    FOREIGN KEY (currency_id) REFERENCES ref.currencies (id);

ALTER TABLE ONLY billing.plan_features
    ADD CONSTRAINT plan_features_plan_id_fkey
    FOREIGN KEY (plan_id) REFERENCES billing.plans (id);

ALTER TABLE ONLY billing.plan_features
    ADD CONSTRAINT plan_features_feature_id_fkey
    FOREIGN KEY (feature_id) REFERENCES billing.features (id);

ALTER TABLE ONLY billing.subscriptions
    ADD CONSTRAINT subscriptions_tenant_id_fkey
    FOREIGN KEY (tenant_id) REFERENCES public.tenants (id);

ALTER TABLE ONLY billing.subscriptions
    ADD CONSTRAINT subscriptions_plan_id_fkey
    FOREIGN KEY (plan_id) REFERENCES billing.plans (id);

ALTER TABLE ONLY billing.subscription_items
    ADD CONSTRAINT subscription_items_subscription_id_fkey
    FOREIGN KEY (subscription_id) REFERENCES billing.subscriptions (id);

ALTER TABLE ONLY billing.subscription_items
    ADD CONSTRAINT subscription_items_feature_id_fkey
    FOREIGN KEY (feature_id) REFERENCES billing.features (id);

ALTER TABLE ONLY billing.invoices
    ADD CONSTRAINT invoices_subscription_id_fkey
    FOREIGN KEY (subscription_id) REFERENCES billing.subscriptions (id);

ALTER TABLE ONLY billing.invoice_lines
    ADD CONSTRAINT invoice_lines_invoice_id_fkey
    FOREIGN KEY (invoice_id) REFERENCES billing.invoices (id);

ALTER TABLE ONLY billing.payments
    ADD CONSTRAINT payments_invoice_id_fkey
    FOREIGN KEY (invoice_id) REFERENCES billing.invoices (id);

ALTER TABLE ONLY billing.payments
    ADD CONSTRAINT payments_method_id_fkey
    FOREIGN KEY (method_id) REFERENCES billing.payment_methods (id);

ALTER TABLE ONLY billing.payment_methods
    ADD CONSTRAINT payment_methods_tenant_id_fkey
    FOREIGN KEY (tenant_id) REFERENCES public.tenants (id);

ALTER TABLE ONLY billing.refunds
    ADD CONSTRAINT refunds_payment_id_fkey
    FOREIGN KEY (payment_id) REFERENCES billing.payments (id);

ALTER TABLE ONLY billing.coupon_redemptions
    ADD CONSTRAINT coupon_redemptions_coupon_id_fkey
    FOREIGN KEY (coupon_id) REFERENCES billing.coupons (id);

ALTER TABLE ONLY billing.coupon_redemptions
    ADD CONSTRAINT coupon_redemptions_subscription_id_fkey
    FOREIGN KEY (subscription_id) REFERENCES billing.subscriptions (id);

ALTER TABLE ONLY billing.credit_notes
    ADD CONSTRAINT credit_notes_invoice_id_fkey
    FOREIGN KEY (invoice_id) REFERENCES billing.invoices (id);

ALTER TABLE ONLY billing.tax_rates
    ADD CONSTRAINT tax_rates_country_id_fkey
    FOREIGN KEY (country_id) REFERENCES ref.countries (id);

ALTER TABLE ONLY billing.invoice_taxes
    ADD CONSTRAINT invoice_taxes_invoice_id_fkey
    FOREIGN KEY (invoice_id) REFERENCES billing.invoices (id);

ALTER TABLE ONLY billing.invoice_taxes
    ADD CONSTRAINT invoice_taxes_tax_rate_id_fkey
    FOREIGN KEY (tax_rate_id) REFERENCES billing.tax_rates (id);

ALTER TABLE ONLY billing.usage_records
    ADD CONSTRAINT usage_records_subscription_item_id_fkey
    FOREIGN KEY (subscription_item_id) REFERENCES billing.subscription_items (id);

ALTER TABLE ONLY public.workspaces
    ADD CONSTRAINT workspaces_tenant_id_fkey
    FOREIGN KEY (tenant_id) REFERENCES public.tenants (id);

ALTER TABLE ONLY public.projects
    ADD CONSTRAINT projects_workspace_id_fkey
    FOREIGN KEY (workspace_id) REFERENCES public.workspaces (id);

ALTER TABLE ONLY public.projects
    ADD CONSTRAINT projects_owner_id_fkey
    FOREIGN KEY (owner_id) REFERENCES public.users (id);

ALTER TABLE ONLY public.project_members
    ADD CONSTRAINT project_members_project_id_fkey
    FOREIGN KEY (project_id) REFERENCES public.projects (id);

ALTER TABLE ONLY public.project_members
    ADD CONSTRAINT project_members_user_id_fkey
    FOREIGN KEY (user_id) REFERENCES public.users (id);

ALTER TABLE ONLY public.project_members
    ADD CONSTRAINT project_members_role_id_fkey
    FOREIGN KEY (role_id) REFERENCES public.roles (id);

ALTER TABLE ONLY public.documents
    ADD CONSTRAINT documents_project_id_fkey
    FOREIGN KEY (project_id) REFERENCES public.projects (id);

ALTER TABLE ONLY public.documents
    ADD CONSTRAINT documents_author_id_fkey
    FOREIGN KEY (author_id) REFERENCES public.users (id);

ALTER TABLE ONLY public.documents
    ADD CONSTRAINT documents_folder_id_fkey
    FOREIGN KEY (folder_id) REFERENCES public.folders (id);

ALTER TABLE ONLY public.folders
    ADD CONSTRAINT folders_project_id_fkey
    FOREIGN KEY (project_id) REFERENCES public.projects (id);

ALTER TABLE ONLY public.document_revisions
    ADD CONSTRAINT document_revisions_document_id_fkey
    FOREIGN KEY (document_id) REFERENCES public.documents (id);

ALTER TABLE ONLY public.document_revisions
    ADD CONSTRAINT document_revisions_author_id_fkey
    FOREIGN KEY (author_id) REFERENCES public.users (id);

ALTER TABLE ONLY public.comments
    ADD CONSTRAINT comments_document_id_fkey
    FOREIGN KEY (document_id) REFERENCES public.documents (id);

ALTER TABLE ONLY public.comments
    ADD CONSTRAINT comments_author_id_fkey
    FOREIGN KEY (author_id) REFERENCES public.users (id);

ALTER TABLE ONLY public.comment_reactions
    ADD CONSTRAINT comment_reactions_comment_id_fkey
    FOREIGN KEY (comment_id) REFERENCES public.comments (id);

ALTER TABLE ONLY public.comment_reactions
    ADD CONSTRAINT comment_reactions_user_id_fkey
    FOREIGN KEY (user_id) REFERENCES public.users (id);

ALTER TABLE ONLY public.attachments
    ADD CONSTRAINT attachments_document_id_fkey
    FOREIGN KEY (document_id) REFERENCES public.documents (id);

ALTER TABLE ONLY public.attachments
    ADD CONSTRAINT attachments_comment_id_fkey
    FOREIGN KEY (comment_id) REFERENCES public.comments (id);

ALTER TABLE ONLY public.attachments
    ADD CONSTRAINT attachments_blob_id_fkey
    FOREIGN KEY (blob_id) REFERENCES public.blobs (id);

ALTER TABLE ONLY public.blobs
    ADD CONSTRAINT blobs_storage_id_fkey
    FOREIGN KEY (storage_id) REFERENCES public.storage_backends (id);

ALTER TABLE ONLY public.tags
    ADD CONSTRAINT tags_tenant_id_fkey
    FOREIGN KEY (tenant_id) REFERENCES public.tenants (id);

ALTER TABLE ONLY public.taggings
    ADD CONSTRAINT taggings_tag_id_fkey
    FOREIGN KEY (tag_id) REFERENCES public.tags (id);

ALTER TABLE ONLY public.taggings
    ADD CONSTRAINT taggings_document_id_fkey
    FOREIGN KEY (document_id) REFERENCES public.documents (id);

ALTER TABLE ONLY public.document_shares
    ADD CONSTRAINT document_shares_document_id_fkey
    FOREIGN KEY (document_id) REFERENCES public.documents (id);

ALTER TABLE ONLY public.document_shares
    ADD CONSTRAINT document_shares_shared_with_fkey
    FOREIGN KEY (shared_with) REFERENCES public.users (id);

ALTER TABLE ONLY public.templates
    ADD CONSTRAINT templates_tenant_id_fkey
    FOREIGN KEY (tenant_id) REFERENCES public.tenants (id);

ALTER TABLE ONLY public.favorites
    ADD CONSTRAINT favorites_user_id_fkey
    FOREIGN KEY (user_id) REFERENCES public.users (id);

ALTER TABLE ONLY public.favorites
    ADD CONSTRAINT favorites_document_id_fkey
    FOREIGN KEY (document_id) REFERENCES public.documents (id);

ALTER TABLE ONLY public.boards
    ADD CONSTRAINT boards_project_id_fkey
    FOREIGN KEY (project_id) REFERENCES public.projects (id);

ALTER TABLE ONLY public.board_columns
    ADD CONSTRAINT board_columns_board_id_fkey
    FOREIGN KEY (board_id) REFERENCES public.boards (id);

ALTER TABLE ONLY public.cards
    ADD CONSTRAINT cards_column_id_fkey
    FOREIGN KEY (column_id) REFERENCES public.board_columns (id);

ALTER TABLE ONLY public.cards
    ADD CONSTRAINT cards_assignee_id_fkey
    FOREIGN KEY (assignee_id) REFERENCES public.users (id);

ALTER TABLE ONLY public.cards
    ADD CONSTRAINT cards_document_id_fkey
    FOREIGN KEY (document_id) REFERENCES public.documents (id);

ALTER TABLE ONLY public.card_labels
    ADD CONSTRAINT card_labels_card_id_fkey
    FOREIGN KEY (card_id) REFERENCES public.cards (id);

ALTER TABLE ONLY public.card_labels
    ADD CONSTRAINT card_labels_label_id_fkey
    FOREIGN KEY (label_id) REFERENCES public.labels (id);

ALTER TABLE ONLY public.labels
    ADD CONSTRAINT labels_project_id_fkey
    FOREIGN KEY (project_id) REFERENCES public.projects (id);

ALTER TABLE ONLY public.checklists
    ADD CONSTRAINT checklists_card_id_fkey
    FOREIGN KEY (card_id) REFERENCES public.cards (id);

ALTER TABLE ONLY public.checklist_items
    ADD CONSTRAINT checklist_items_checklist_id_fkey
    FOREIGN KEY (checklist_id) REFERENCES public.checklists (id);

ALTER TABLE ONLY public.card_watchers
    ADD CONSTRAINT card_watchers_card_id_fkey
    FOREIGN KEY (card_id) REFERENCES public.cards (id);

ALTER TABLE ONLY public.card_watchers
    ADD CONSTRAINT card_watchers_user_id_fkey
    FOREIGN KEY (user_id) REFERENCES public.users (id);

ALTER TABLE ONLY public.time_entries
    ADD CONSTRAINT time_entries_card_id_fkey
    FOREIGN KEY (card_id) REFERENCES public.cards (id);

ALTER TABLE ONLY public.time_entries
    ADD CONSTRAINT time_entries_user_id_fkey
    FOREIGN KEY (user_id) REFERENCES public.users (id);

ALTER TABLE ONLY public.milestones
    ADD CONSTRAINT milestones_project_id_fkey
    FOREIGN KEY (project_id) REFERENCES public.projects (id);

ALTER TABLE ONLY public.card_milestones
    ADD CONSTRAINT card_milestones_card_id_fkey
    FOREIGN KEY (card_id) REFERENCES public.cards (id);

ALTER TABLE ONLY public.card_milestones
    ADD CONSTRAINT card_milestones_milestone_id_fkey
    FOREIGN KEY (milestone_id) REFERENCES public.milestones (id);

ALTER TABLE ONLY public.automations
    ADD CONSTRAINT automations_board_id_fkey
    FOREIGN KEY (board_id) REFERENCES public.boards (id);

ALTER TABLE ONLY public.channels
    ADD CONSTRAINT channels_workspace_id_fkey
    FOREIGN KEY (workspace_id) REFERENCES public.workspaces (id);

ALTER TABLE ONLY public.channel_members
    ADD CONSTRAINT channel_members_channel_id_fkey
    FOREIGN KEY (channel_id) REFERENCES public.channels (id);

ALTER TABLE ONLY public.channel_members
    ADD CONSTRAINT channel_members_user_id_fkey
    FOREIGN KEY (user_id) REFERENCES public.users (id);

ALTER TABLE ONLY public.messages
    ADD CONSTRAINT messages_channel_id_fkey
    FOREIGN KEY (channel_id) REFERENCES public.channels (id);

ALTER TABLE ONLY public.messages
    ADD CONSTRAINT messages_author_id_fkey
    FOREIGN KEY (author_id) REFERENCES public.users (id);

ALTER TABLE ONLY public.messages
    ADD CONSTRAINT messages_thread_id_fkey
    FOREIGN KEY (thread_id) REFERENCES public.threads (id);

ALTER TABLE ONLY public.threads
    ADD CONSTRAINT threads_channel_id_fkey
    FOREIGN KEY (channel_id) REFERENCES public.channels (id);

ALTER TABLE ONLY public.message_reactions
    ADD CONSTRAINT message_reactions_message_id_fkey
    FOREIGN KEY (message_id) REFERENCES public.messages (id);

ALTER TABLE ONLY public.message_reactions
    ADD CONSTRAINT message_reactions_user_id_fkey
    FOREIGN KEY (user_id) REFERENCES public.users (id);

ALTER TABLE ONLY public.mentions
    ADD CONSTRAINT mentions_message_id_fkey
    FOREIGN KEY (message_id) REFERENCES public.messages (id);

ALTER TABLE ONLY public.mentions
    ADD CONSTRAINT mentions_user_id_fkey
    FOREIGN KEY (user_id) REFERENCES public.users (id);

ALTER TABLE ONLY public.read_receipts
    ADD CONSTRAINT read_receipts_message_id_fkey
    FOREIGN KEY (message_id) REFERENCES public.messages (id);

ALTER TABLE ONLY public.read_receipts
    ADD CONSTRAINT read_receipts_user_id_fkey
    FOREIGN KEY (user_id) REFERENCES public.users (id);

ALTER TABLE ONLY public.notifications
    ADD CONSTRAINT notifications_user_id_fkey
    FOREIGN KEY (user_id) REFERENCES public.users (id);

ALTER TABLE ONLY public.notifications
    ADD CONSTRAINT notifications_channel_id_fkey
    FOREIGN KEY (channel_id) REFERENCES public.channels (id);

ALTER TABLE ONLY public.notification_preferences
    ADD CONSTRAINT notification_preferences_user_id_fkey
    FOREIGN KEY (user_id) REFERENCES public.users (id);

ALTER TABLE ONLY public.email_deliveries
    ADD CONSTRAINT email_deliveries_user_id_fkey
    FOREIGN KEY (user_id) REFERENCES public.users (id);

ALTER TABLE ONLY public.email_deliveries
    ADD CONSTRAINT email_deliveries_template_id_fkey
    FOREIGN KEY (template_id) REFERENCES public.email_templates (id);

ALTER TABLE ONLY public.integrations
    ADD CONSTRAINT integrations_tenant_id_fkey
    FOREIGN KEY (tenant_id) REFERENCES public.tenants (id);

ALTER TABLE ONLY public.integration_configs
    ADD CONSTRAINT integration_configs_integration_id_fkey
    FOREIGN KEY (integration_id) REFERENCES public.integrations (id);

ALTER TABLE ONLY public.webhooks
    ADD CONSTRAINT webhooks_tenant_id_fkey
    FOREIGN KEY (tenant_id) REFERENCES public.tenants (id);

ALTER TABLE ONLY public.webhooks
    ADD CONSTRAINT webhooks_integration_id_fkey
    FOREIGN KEY (integration_id) REFERENCES public.integrations (id);

ALTER TABLE ONLY public.webhook_deliveries
    ADD CONSTRAINT webhook_deliveries_webhook_id_fkey
    FOREIGN KEY (webhook_id) REFERENCES public.webhooks (id);

ALTER TABLE ONLY public.webhook_events
    ADD CONSTRAINT webhook_events_webhook_id_fkey
    FOREIGN KEY (webhook_id) REFERENCES public.webhooks (id);

ALTER TABLE ONLY public.webhook_events
    ADD CONSTRAINT webhook_events_event_type_id_fkey
    FOREIGN KEY (event_type_id) REFERENCES public.event_types (id);

ALTER TABLE ONLY public.imports
    ADD CONSTRAINT imports_tenant_id_fkey
    FOREIGN KEY (tenant_id) REFERENCES public.tenants (id);

ALTER TABLE ONLY public.import_rows
    ADD CONSTRAINT import_rows_import_id_fkey
    FOREIGN KEY (import_id) REFERENCES public.imports (id);

ALTER TABLE ONLY public.exports
    ADD CONSTRAINT exports_tenant_id_fkey
    FOREIGN KEY (tenant_id) REFERENCES public.tenants (id);

ALTER TABLE ONLY public.exports
    ADD CONSTRAINT exports_requested_by_fkey
    FOREIGN KEY (requested_by) REFERENCES public.users (id);

ALTER TABLE ONLY public.exports
    ADD CONSTRAINT exports_blob_id_fkey
    FOREIGN KEY (blob_id) REFERENCES public.blobs (id);

ALTER TABLE ONLY audit.audit_logs
    ADD CONSTRAINT audit_logs_tenant_id_fkey
    FOREIGN KEY (tenant_id) REFERENCES public.tenants (id);

ALTER TABLE ONLY audit.audit_logs
    ADD CONSTRAINT audit_logs_actor_id_fkey
    FOREIGN KEY (actor_id) REFERENCES public.users (id);

ALTER TABLE ONLY audit.audit_logs
    ADD CONSTRAINT audit_logs_event_type_id_fkey
    FOREIGN KEY (event_type_id) REFERENCES public.event_types (id);

ALTER TABLE ONLY audit.audit_changes
    ADD CONSTRAINT audit_changes_audit_log_id_fkey
    FOREIGN KEY (audit_log_id) REFERENCES audit.audit_logs (id);

ALTER TABLE ONLY audit.login_attempts
    ADD CONSTRAINT login_attempts_user_id_fkey
    FOREIGN KEY (user_id) REFERENCES public.users (id);

ALTER TABLE ONLY audit.security_alerts
    ADD CONSTRAINT security_alerts_tenant_id_fkey
    FOREIGN KEY (tenant_id) REFERENCES public.tenants (id);

ALTER TABLE ONLY audit.security_alerts
    ADD CONSTRAINT security_alerts_audit_log_id_fkey
    FOREIGN KEY (audit_log_id) REFERENCES audit.audit_logs (id);

ALTER TABLE ONLY ops.job_attempts
    ADD CONSTRAINT job_attempts_job_id_fkey
    FOREIGN KEY (job_id) REFERENCES ops.jobs (id);

ALTER TABLE ONLY ops.schedules
    ADD CONSTRAINT schedules_job_id_fkey
    FOREIGN KEY (job_id) REFERENCES ops.jobs (id);

ALTER TABLE ONLY ops.feature_flags
    ADD CONSTRAINT feature_flags_tenant_id_fkey
    FOREIGN KEY (tenant_id) REFERENCES public.tenants (id);

ALTER TABLE ONLY ops.feature_flags
    ADD CONSTRAINT feature_flags_feature_id_fkey
    FOREIGN KEY (feature_id) REFERENCES billing.features (id);

ALTER TABLE ONLY ops.rate_limits
    ADD CONSTRAINT rate_limits_api_key_id_fkey
    FOREIGN KEY (api_key_id) REFERENCES public.api_keys (id);

ALTER TABLE ONLY ops.incident_updates
    ADD CONSTRAINT incident_updates_incident_id_fkey
    FOREIGN KEY (incident_id) REFERENCES ops.service_incidents (id);

ALTER TABLE ONLY ops.metrics_daily
    ADD CONSTRAINT metrics_daily_tenant_id_fkey
    FOREIGN KEY (tenant_id) REFERENCES public.tenants (id);

ALTER TABLE ONLY ops.search_indexes
    ADD CONSTRAINT search_indexes_document_id_fkey
    FOREIGN KEY (document_id) REFERENCES public.documents (id);

ALTER TABLE ONLY ref.holidays
    ADD CONSTRAINT holidays_country_id_fkey
    FOREIGN KEY (country_id) REFERENCES ref.countries (id);


--
-- Indexes.
--

CREATE INDEX tenants_plan_id_idx ON public.tenants (plan_id);

CREATE INDEX users_tenant_id_idx ON public.users (tenant_id);

CREATE INDEX teams_tenant_id_idx ON public.teams (tenant_id);

CREATE INDEX team_members_team_id_idx ON public.team_members (team_id);

CREATE INDEX team_members_user_id_idx ON public.team_members (user_id);

CREATE INDEX team_members_role_id_idx ON public.team_members (role_id);

CREATE INDEX roles_tenant_id_idx ON public.roles (tenant_id);

CREATE INDEX role_permissions_role_id_idx ON public.role_permissions (role_id);

CREATE INDEX role_permissions_permission_id_idx ON public.role_permissions (permission_id);

CREATE INDEX invitations_tenant_id_idx ON public.invitations (tenant_id);

CREATE INDEX invitations_invited_by_idx ON public.invitations (invited_by);

CREATE INDEX invitations_role_id_idx ON public.invitations (role_id);

CREATE INDEX user_profiles_user_id_idx ON public.user_profiles (user_id);

CREATE INDEX user_profiles_country_id_idx ON public.user_profiles (country_id);

CREATE INDEX user_settings_user_id_idx ON public.user_settings (user_id);

CREATE INDEX user_settings_locale_id_idx ON public.user_settings (locale_id);

CREATE INDEX user_settings_timezone_id_idx ON public.user_settings (timezone_id);

CREATE INDEX sessions_user_id_idx ON public.sessions (user_id);

CREATE INDEX api_keys_tenant_id_idx ON public.api_keys (tenant_id);

CREATE INDEX api_keys_created_by_idx ON public.api_keys (created_by);

CREATE INDEX api_key_scopes_api_key_id_idx ON public.api_key_scopes (api_key_id);

CREATE INDEX api_key_scopes_permission_id_idx ON public.api_key_scopes (permission_id);

CREATE INDEX oauth_identities_user_id_idx ON public.oauth_identities (user_id);

CREATE INDEX oauth_identities_provider_id_idx ON public.oauth_identities (provider_id);

CREATE INDEX plans_currency_id_idx ON billing.plans (currency_id);

CREATE INDEX plan_features_plan_id_idx ON billing.plan_features (plan_id);

CREATE INDEX plan_features_feature_id_idx ON billing.plan_features (feature_id);

CREATE INDEX subscriptions_tenant_id_idx ON billing.subscriptions (tenant_id);

CREATE INDEX subscriptions_plan_id_idx ON billing.subscriptions (plan_id);

CREATE INDEX subscription_items_subscription_id_idx ON billing.subscription_items (subscription_id);

CREATE INDEX subscription_items_feature_id_idx ON billing.subscription_items (feature_id);

CREATE INDEX invoices_subscription_id_idx ON billing.invoices (subscription_id);

CREATE INDEX invoice_lines_invoice_id_idx ON billing.invoice_lines (invoice_id);

CREATE INDEX payments_invoice_id_idx ON billing.payments (invoice_id);

CREATE INDEX payments_method_id_idx ON billing.payments (method_id);

CREATE INDEX payment_methods_tenant_id_idx ON billing.payment_methods (tenant_id);

CREATE INDEX refunds_payment_id_idx ON billing.refunds (payment_id);

CREATE INDEX coupon_redemptions_coupon_id_idx ON billing.coupon_redemptions (coupon_id);

CREATE INDEX coupon_redemptions_subscription_id_idx ON billing.coupon_redemptions (subscription_id);

CREATE INDEX credit_notes_invoice_id_idx ON billing.credit_notes (invoice_id);

CREATE INDEX tax_rates_country_id_idx ON billing.tax_rates (country_id);

CREATE INDEX invoice_taxes_invoice_id_idx ON billing.invoice_taxes (invoice_id);

CREATE INDEX invoice_taxes_tax_rate_id_idx ON billing.invoice_taxes (tax_rate_id);

CREATE INDEX usage_records_subscription_item_id_idx ON billing.usage_records (subscription_item_id);

CREATE INDEX workspaces_tenant_id_idx ON public.workspaces (tenant_id);

CREATE INDEX projects_workspace_id_idx ON public.projects (workspace_id);

CREATE INDEX projects_owner_id_idx ON public.projects (owner_id);

CREATE INDEX project_members_project_id_idx ON public.project_members (project_id);

CREATE INDEX project_members_user_id_idx ON public.project_members (user_id);

CREATE INDEX project_members_role_id_idx ON public.project_members (role_id);

CREATE INDEX documents_project_id_idx ON public.documents (project_id);

CREATE INDEX documents_author_id_idx ON public.documents (author_id);

CREATE INDEX documents_folder_id_idx ON public.documents (folder_id);

CREATE INDEX folders_project_id_idx ON public.folders (project_id);

CREATE INDEX document_revisions_document_id_idx ON public.document_revisions (document_id);

CREATE INDEX document_revisions_author_id_idx ON public.document_revisions (author_id);

CREATE INDEX comments_document_id_idx ON public.comments (document_id);

CREATE INDEX comments_author_id_idx ON public.comments (author_id);

CREATE INDEX comment_reactions_comment_id_idx ON public.comment_reactions (comment_id);

CREATE INDEX comment_reactions_user_id_idx ON public.comment_reactions (user_id);

CREATE INDEX attachments_document_id_idx ON public.attachments (document_id);

CREATE INDEX attachments_comment_id_idx ON public.attachments (comment_id);

CREATE INDEX attachments_blob_id_idx ON public.attachments (blob_id);

CREATE INDEX blobs_storage_id_idx ON public.blobs (storage_id);

CREATE INDEX tags_tenant_id_idx ON public.tags (tenant_id);

CREATE INDEX taggings_tag_id_idx ON public.taggings (tag_id);

CREATE INDEX taggings_document_id_idx ON public.taggings (document_id);

CREATE INDEX document_shares_document_id_idx ON public.document_shares (document_id);

CREATE INDEX document_shares_shared_with_idx ON public.document_shares (shared_with);

CREATE INDEX templates_tenant_id_idx ON public.templates (tenant_id);

CREATE INDEX favorites_user_id_idx ON public.favorites (user_id);

CREATE INDEX favorites_document_id_idx ON public.favorites (document_id);

CREATE INDEX boards_project_id_idx ON public.boards (project_id);

CREATE INDEX board_columns_board_id_idx ON public.board_columns (board_id);

CREATE INDEX cards_column_id_idx ON public.cards (column_id);

CREATE INDEX cards_assignee_id_idx ON public.cards (assignee_id);

CREATE INDEX cards_document_id_idx ON public.cards (document_id);

CREATE INDEX card_labels_card_id_idx ON public.card_labels (card_id);

CREATE INDEX card_labels_label_id_idx ON public.card_labels (label_id);

CREATE INDEX labels_project_id_idx ON public.labels (project_id);

CREATE INDEX checklists_card_id_idx ON public.checklists (card_id);

CREATE INDEX checklist_items_checklist_id_idx ON public.checklist_items (checklist_id);

CREATE INDEX card_watchers_card_id_idx ON public.card_watchers (card_id);

CREATE INDEX card_watchers_user_id_idx ON public.card_watchers (user_id);

CREATE INDEX time_entries_card_id_idx ON public.time_entries (card_id);

CREATE INDEX time_entries_user_id_idx ON public.time_entries (user_id);

CREATE INDEX milestones_project_id_idx ON public.milestones (project_id);

CREATE INDEX card_milestones_card_id_idx ON public.card_milestones (card_id);

CREATE INDEX card_milestones_milestone_id_idx ON public.card_milestones (milestone_id);

CREATE INDEX automations_board_id_idx ON public.automations (board_id);

CREATE INDEX channels_workspace_id_idx ON public.channels (workspace_id);

CREATE INDEX channel_members_channel_id_idx ON public.channel_members (channel_id);

CREATE INDEX channel_members_user_id_idx ON public.channel_members (user_id);

CREATE INDEX messages_channel_id_idx ON public.messages (channel_id);

CREATE INDEX messages_author_id_idx ON public.messages (author_id);

CREATE INDEX messages_thread_id_idx ON public.messages (thread_id);

CREATE INDEX threads_channel_id_idx ON public.threads (channel_id);

CREATE INDEX message_reactions_message_id_idx ON public.message_reactions (message_id);

CREATE INDEX message_reactions_user_id_idx ON public.message_reactions (user_id);

CREATE INDEX mentions_message_id_idx ON public.mentions (message_id);

CREATE INDEX mentions_user_id_idx ON public.mentions (user_id);

CREATE INDEX read_receipts_message_id_idx ON public.read_receipts (message_id);

CREATE INDEX read_receipts_user_id_idx ON public.read_receipts (user_id);

CREATE INDEX notifications_user_id_idx ON public.notifications (user_id);

CREATE INDEX notifications_channel_id_idx ON public.notifications (channel_id);

CREATE INDEX notification_preferences_user_id_idx ON public.notification_preferences (user_id);

CREATE INDEX email_deliveries_user_id_idx ON public.email_deliveries (user_id);

CREATE INDEX email_deliveries_template_id_idx ON public.email_deliveries (template_id);

CREATE INDEX integrations_tenant_id_idx ON public.integrations (tenant_id);

CREATE INDEX integration_configs_integration_id_idx ON public.integration_configs (integration_id);

CREATE INDEX webhooks_tenant_id_idx ON public.webhooks (tenant_id);

CREATE INDEX webhooks_integration_id_idx ON public.webhooks (integration_id);

CREATE INDEX webhook_deliveries_webhook_id_idx ON public.webhook_deliveries (webhook_id);

CREATE INDEX webhook_events_webhook_id_idx ON public.webhook_events (webhook_id);

CREATE INDEX webhook_events_event_type_id_idx ON public.webhook_events (event_type_id);

CREATE INDEX imports_tenant_id_idx ON public.imports (tenant_id);

CREATE INDEX import_rows_import_id_idx ON public.import_rows (import_id);

CREATE INDEX exports_tenant_id_idx ON public.exports (tenant_id);

CREATE INDEX exports_requested_by_idx ON public.exports (requested_by);

CREATE INDEX exports_blob_id_idx ON public.exports (blob_id);

CREATE INDEX audit_logs_tenant_id_idx ON audit.audit_logs (tenant_id);

CREATE INDEX audit_logs_actor_id_idx ON audit.audit_logs (actor_id);

CREATE INDEX audit_logs_event_type_id_idx ON audit.audit_logs (event_type_id);

CREATE INDEX audit_changes_audit_log_id_idx ON audit.audit_changes (audit_log_id);

CREATE INDEX login_attempts_user_id_idx ON audit.login_attempts (user_id);

CREATE INDEX security_alerts_tenant_id_idx ON audit.security_alerts (tenant_id);

CREATE INDEX security_alerts_audit_log_id_idx ON audit.security_alerts (audit_log_id);

CREATE INDEX job_attempts_job_id_idx ON ops.job_attempts (job_id);

CREATE INDEX schedules_job_id_idx ON ops.schedules (job_id);

CREATE INDEX feature_flags_tenant_id_idx ON ops.feature_flags (tenant_id);

CREATE INDEX feature_flags_feature_id_idx ON ops.feature_flags (feature_id);

CREATE INDEX rate_limits_api_key_id_idx ON ops.rate_limits (api_key_id);

CREATE INDEX incident_updates_incident_id_idx ON ops.incident_updates (incident_id);

CREATE INDEX metrics_daily_tenant_id_idx ON ops.metrics_daily (tenant_id);

CREATE INDEX search_indexes_document_id_idx ON ops.search_indexes (document_id);

CREATE INDEX holidays_country_id_idx ON ref.holidays (country_id);
