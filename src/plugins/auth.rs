use crate::fs;
use crate::logger::file_msg;
use crate::plugins::InstallConfig;
use crate::plugins::Plugin;
use crate::project::add_dependency;
use anyhow::Result;
use indoc::indoc;
use rust_embed::RustEmbed;

pub struct Auth {}

#[derive(RustEmbed)]
#[folder = "template-plugin-auth"]
struct Asset;

impl Plugin for Auth {
    fn name(&self) -> &'static str {
        "Auth"
    }

    fn install(&self, install_config: InstallConfig) -> Result<()> {
        for filename in Asset::iter() {
            let file_contents = Asset::get(filename.as_ref()).unwrap();
            let mut file_path = std::path::PathBuf::from(&install_config.project_dir);
            file_path.push(filename.as_ref());
            let mut directory_path = std::path::PathBuf::from(&file_path);
            directory_path.pop();

            file_msg(filename.as_ref());
            std::fs::create_dir_all(directory_path)?;
            std::fs::write(file_path, file_contents)?;
        }

        /* Add dependencies */
        add_dependency(
            &install_config.project_dir,
            "argonautica",
            toml::Value::String("0.2.0".into()),
        )?;

        // TODO: Fix these appends/prepends by prepending the filepath with project_dir
        // currently, this works because we assume the current working directory is the project's root
        fs::prepend(
            "frontend/src/App.tsx",
            r#"import { useAuth, useAuthCheck } from './hooks/useAuth'
  import { AccountPage } from './containers/AccountPage'
  import { LoginPage } from './containers/LoginPage'
  import { ActivationPage } from './containers/ActivationPage'
  import { RegistrationPage } from './containers/RegistrationPage'
  import { RecoveryPage } from './containers/RecoveryPage'
  import { ResetPage } from './containers/ResetPage'    
    "#,
        )?;
        fs::prepend(
            "frontend/src/index.tsx",
            "import { AuthProvider } from './hooks/useAuth'",
        )?;
        fs::replace(
            "frontend/src/App.tsx",
            "const App = () => {",
            r#"const App = () => {
  useAuthCheck()
  const auth = useAuth()
    "#,
        )?;
        fs::replace(
            "frontend/src/App.tsx",
            "{/* CRA: routes */}",
            r#"{/* CRA: routes */}
        <Route path="/login"><LoginPage /></Route>
        <Route path="/recovery"><RecoveryPage /></Route>
        <Route path="/reset"><ResetPage /></Route>
        <Route path="/activate"><ActivationPage /></Route>
        <Route path="/register"><RegistrationPage /></Route>
        <Route path="/account"><AccountPage /></Route>
    "#,
        )?;
        fs::replace(
            "frontend/src/App.tsx",
            "{/* CRA: left-aligned nav buttons */}",
            r#"{/* CRA: left-aligned nav buttons */}
          <a className="NavButton" onClick={() => history.push('/account')}>Account</a>
    "#,
        )?;
        fs::replace(
            "frontend/src/App.tsx",
            "{/* CRA: right-aligned nav buttons */}",
            r#"{/* CRA: right-aligned nav buttons */}
          { auth.isAuthenticated && <a className="NavButton" onClick={() => auth.logout()}>Logout</a> }
          { !auth.isAuthenticated && <a className="NavButton" onClick={() => history.push('/login')}>Login/Register</a> }
    "#,
        )?;
        fs::replace(
            "frontend/src/index.tsx",
            "{/* CRA: Wrap */}",
            "{/* CRA: Wrap */}\n<AuthProvider>",
        )?;
        fs::replace(
            "frontend/src/index.tsx",
            "{/* CRA: Unwrap */}",
            "{/* CRA: Unwrap */}\n</AuthProvider>",
        )?;
        fs::append("backend/extractors/mod.rs", "\npub mod auth;")?;
        fs::append("backend/mail/mod.rs", "\npub mod auth_register;")?;
        fs::append("backend/mail/mod.rs", "\npub mod auth_activated;")?;
        fs::append(
            "backend/mail/mod.rs",
            "\npub mod auth_recover_existent_account;",
        )?;
        fs::append(
            "backend/mail/mod.rs",
            "\npub mod auth_recover_nonexistent_account;",
        )?;
        fs::append("backend/mail/mod.rs", "\npub mod auth_password_changed;")?;
        fs::append("backend/mail/mod.rs", "\npub mod auth_password_reset;")?;
        fs::append("backend/models/mod.rs", "\npub mod user;")?;
        fs::append("backend/models/mod.rs", "\npub mod user_session;")?;
        fs::append("backend/models/mod.rs", "\npub mod permissions;")?;
        fs::append("backend/services/mod.rs", "\npub mod auth;")?;

        crate::db::create_migration(
            "plugin_auth",
            indoc! {r#"
      CREATE TABLE users (
        id SERIAL PRIMARY KEY,
        email TEXT NOT NULL,
        hash_password TEXT NOT NULL,
        activated BOOL NOT NULL DEFAULT FALSE,
        created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
        updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
      );
      
      SELECT diesel_manage_updated_at('users');
      
      CREATE TABLE user_sessions (
        id SERIAL PRIMARY KEY,
        user_id SERIAL NOT NULL REFERENCES users(id),
        refresh_token TEXT NOT NULL,
        device TEXT,
        created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
        updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
      );
      
      SELECT diesel_manage_updated_at('user_sessions');

      CREATE TABLE user_permissions (
        user_id SERIAL NOT NULL REFERENCES users(id),
        permission TEXT NOT NULL,
        created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
        PRIMARY KEY (user_id, permission)
      );
      
      CREATE TABLE user_roles (
        user_id SERIAL NOT NULL REFERENCES users(id),
        role TEXT NOT NULL,
        created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
        PRIMARY KEY (user_id, role)
      );
      
      CREATE TABLE role_permissions (
        role TEXT NOT NULL PRIMARY KEY,
        permission TEXT NOT NULL,
        created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
        UNIQUE (role, permission)
      );      
    "#},
            indoc! {r#"
      DROP TABLE users CASCADE ALL;
      DROP TABLE user_sessions CASCADE ALL;
      DROP TABLE user_permissions CASCADE ALL;
      DROP TABLE role_permissions CASCADE ALL;
      DROP TABLE user_roles CASCADE ALL;
    "#},
        )?;

        crate::service::register_service("auth", "/auth")?;

        Ok(())
    }
}
