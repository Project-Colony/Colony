use std::collections::HashMap;
use std::sync::OnceLock;

static LOCALE: OnceLock<Locale> = OnceLock::new();

pub struct Locale {
    strings: HashMap<String, String>,
    lang: String,
}

impl Locale {
    fn new(lang: &str) -> Self {
        let mut strings = HashMap::new();

        match lang {
            "fr" => {
                // Sidebar
                strings.insert("categories".into(), "Catégories".into());
                strings.insert("rescan".into(), "Rescan".into());

                // GitHub panel
                strings.insert("github_connect_desc".into(), "Connectez-vous à GitHub pour détecter les dépôts Colony (colony.json) du compte MotherSphere.".into());
                strings.insert("github_login".into(), "Se connecter avec GitHub".into());
                strings.insert("github_public_api".into(), "Mode non connecté : API publique GitHub (60 req/h)".into());
                strings.insert("github_rate_limit".into(), "Quota GitHub atteint. Réessayez dans {wait} secondes.".into());
                strings.insert("github_enter_code".into(), "Entrez ce code sur GitHub :".into());
                strings.insert("github_copy_hint".into(), "Cliquez pour copier — En attente d'autorisation...".into());
                strings.insert("github_connecting".into(), "Connexion en cours...".into());
                strings.insert("github_connected".into(), "Connecté".into());
                strings.insert("github_repos_detected".into(), "{count} dépôts Colony détectés".into());
                strings.insert("github_no_repos".into(), "Aucun dépôt avec colony.json trouvé.".into());
                strings.insert("github_refresh".into(), "Rafraîchir les dépôts".into());
                strings.insert("github_logout".into(), "Se déconnecter".into());
                strings.insert("github_error".into(), "Erreur : {error}".into());
                strings.insert("github_retry".into(), "Réessayer".into());
                strings.insert("github_disconnected".into(), "Déconnecté de GitHub".into());

                // App grid
                strings.insert("no_apps_found".into(), "Aucune application trouvée".into());
                strings.insert("search_placeholder".into(), "Rechercher des applications...".into());

                // Detail view
                strings.insert("back".into(), "Retour".into());
                strings.insert("language_label".into(), "Langage: {lang}".into());
                strings.insert("launch".into(), "Lancer {name}".into());
                strings.insert("update".into(), "Mettre à jour".into());
                strings.insert("download".into(), "Télécharger".into());
                strings.insert("no_release".into(), "Aucune release disponible".into());
                strings.insert("no_release_platform".into(), "Non disponible pour votre plateforme".into());

                // Status messages
                strings.insert("apps_found".into(), "{count} applications trouvées".into());
                strings.insert("app_launched".into(), "Application lancée.".into());
                strings.insert("installed".into(), "Installé : {path}".into());
                strings.insert("download_error".into(), "Erreur téléchargement : {error}".into());
                strings.insert("downloading".into(), "Téléchargement de {file}…".into());
                strings.insert("no_release_for".into(), "Pas de release pour {platform}".into());
                strings.insert("uninstalled".into(), "{name} désinstallé.".into());
                strings.insert("launch_error".into(), "Impossible de lancer: {error}".into());
                strings.insert("launch_error_empty".into(), "Impossible de lancer: commande vide".into());
                strings.insert("uninstall_error".into(), "Erreur désinstallation : {error}".into());

                // OAuth errors
                strings.insert("oauth_error".into(), "Erreur OAuth: {error}".into());
                strings.insert("github_api_error".into(), "Erreur GitHub: {error}".into());
                strings.insert("scan_error".into(), "Erreur: {error}".into());
                strings.insert("launch_error_msg".into(), "Erreur lancement : {error}".into());
                strings.insert("updates_available".into(), "{count} mise(s) à jour disponible(s) : {names}".into());

                // Thread errors
                strings.insert("error_thread_panic".into(), "Erreur interne : le thread a paniqué".into());

                // Download cancellation
                strings.insert("download_cancelled".into(), "Téléchargement annulé".into());

                // Uninstall confirmation
                strings.insert("confirm_uninstall".into(), "Voulez-vous vraiment désinstaller « {name} » ? Cette action est irréversible.".into());
                strings.insert("cancel".into(), "Annuler".into());
                strings.insert("confirm_delete".into(), "Désinstaller".into());

                // Favorites
                strings.insert("add_favorite".into(), "Ajouter aux favoris".into());
                strings.insert("remove_favorite".into(), "Retirer des favoris".into());

                // First launch — carousel (3 steps)
                strings.insert("welcome_title".into(), "Bienvenue dans Colony".into());
                strings.insert("welcome_desc".into(), "Le lanceur centralisé de l'écosystème MotherSphere. Découvrez, installez et lancez vos apps en un clic.".into());
                // Step 1 — interface tour
                strings.insert("welcome_step1_title".into(), "L'interface en 3 zones".into());
                strings.insert("welcome_step1_tip1_title".into(), "Sidebar".into());
                strings.insert("welcome_step1_tip1_desc".into(), "Filtrez par catégorie ou origine (Colony / système).".into());
                strings.insert("welcome_step1_tip2_title".into(), "Recherche".into());
                strings.insert("welcome_step1_tip2_desc".into(), "Tapez le nom d'une app dans la barre en haut pour filtrer instantanément.".into());
                strings.insert("welcome_step1_tip3_title".into(), "Détail".into());
                strings.insert("welcome_step1_tip3_desc".into(), "Cliquez une app pour lire le README, le changelog et l'installer.".into());
                // Step 2 — GitHub + ready
                strings.insert("welcome_step2_title".into(), "Connectez GitHub (optionnel)".into());
                strings.insert("welcome_step2_desc".into(), "Sans compte : 60 requêtes GitHub par heure. Avec compte : 5000/h + accès aux repos privés. Recommandé si vous comptez explorer beaucoup.".into());
                strings.insert("welcome_step2_hint1".into(), "\u{f005}  Favoris (⭐) pour un accès rapide".into());
                strings.insert("welcome_step2_hint2".into(), "\u{f53f}  24 familles de thèmes dans les préférences".into());
                strings.insert("welcome_step2_hint3".into(), "\u{f059}  Consultez la FAQ et le tutoriel complet sur GitHub".into());
                // Navigation
                strings.insert("welcome_start".into(), "C'est parti !".into());
                strings.insert("welcome_next".into(), "Suivant".into());
                strings.insert("welcome_back".into(), "Retour".into());
                strings.insert("welcome_skip".into(), "Passer".into());
                strings.insert("welcome_connect_now".into(), "Connecter maintenant".into());
                strings.insert("welcome_later".into(), "Plus tard".into());

                // Loading / async feedback
                strings.insert("loading".into(), "Chargement...".into());
                strings.insert("scanning".into(), "Analyse en cours...".into());
                strings.insert("checking_updates".into(), "Vérification des mises à jour...".into());
                strings.insert("syncing_repos".into(), "Synchronisation des dépôts...".into());
                strings.insert("no_results_for".into(), "Aucun résultat pour « {query} »".into());
                strings.insert("n_results_found".into(), "{count} résultat(s) pour « {query} »".into());
                strings.insert("language_restart_notice".into(), "Le changement de langue prendra effet au prochain lancement.".into());
                strings.insert("theme_applied".into(), "Thème appliqué.".into());

                // Keyboard shortcuts
                strings.insert("shortcuts_title".into(), "Raccourcis clavier".into());
                strings.insert("shortcut_esc".into(), "Échap — Fermer le panneau actif".into());
                strings.insert("shortcut_tab".into(), "Tab / Maj+Tab — Naviguer entre les catégories".into());
                strings.insert("shortcut_arrows".into(), "↑ ↓ — Naviguer dans les paramètres".into());
                strings.insert("shortcut_enter".into(), "Entrée — Ouvrir le premier élément visible".into());
                strings.insert("shortcut_pageupdown".into(), "Page ↑/↓ — Naviguer plus vite dans les paramètres".into());

                // Tooltips / hints
                strings.insert("hint_settings".into(), "Ouvrir les préférences".into());
                strings.insert("hint_search".into(), "Tapez pour filtrer les applications".into());
                strings.insert("hint_favorites".into(), "Cliquez sur l'étoile pour ajouter aux favoris".into());
                strings.insert("hint_keyboard".into(), "Utilisez Tab et les flèches pour naviguer".into());

                // Settings
                strings.insert("settings_title".into(), "Préférences".into());
                strings.insert("settings_close".into(), "Fermer".into());
                strings.insert("settings_cat_general".into(), "Général".into());
                strings.insert("settings_cat_appearance".into(), "Apparences".into());
                strings.insert("settings_cat_accessibility".into(), "Accessibilité".into());
                strings.insert("settings_cat_storage".into(), "Stockage".into());
                strings.insert("settings_cat_about".into(), "À propos".into());
                strings.insert("settings_cat_shortcuts".into(), "Raccourcis".into());
                // General
                strings.insert("settings_general_title".into(), "Paramètres généraux".into());
                strings.insert("settings_general_desc".into(), "Les préférences sont enregistrées automatiquement.".into());
                // Startup
                strings.insert("settings_section_startup".into(), "Démarrage".into());
                strings.insert("settings_startup_section_desc".into(), "Gérez l'ouverture de Colony et la restauration des sessions.".into());
                strings.insert("settings_auto_scan".into(), "Scanner au démarrage".into());
                strings.insert("settings_auto_scan_desc".into(), "Analyse les dossiers dès l'ouverture de Colony.".into());
                strings.insert("settings_restore_session".into(), "Restaurer la dernière session".into());
                strings.insert("settings_restore_session_desc".into(), "Catégorie et écran affichés au dernier usage.".into());
                strings.insert("settings_default_view".into(), "Ouvrir sur".into());
                strings.insert("settings_default_view_desc".into(), "Choisissez l'écran par défaut.".into());
                strings.insert("settings_default_view_all".into(), "Toutes".into());
                strings.insert("settings_default_view_favorites".into(), "Favoris".into());
                strings.insert("settings_default_view_recent".into(), "Récents".into());
                strings.insert("settings_close_behavior".into(), "Comportement à la fermeture".into());
                strings.insert("settings_close_behavior_desc".into(), "Choisissez l'action à la fermeture.".into());
                strings.insert("settings_close_quit".into(), "Quitter".into());
                strings.insert("settings_close_tray".into(), "Réduire dans la barre".into());
                // Language
                strings.insert("settings_section_language".into(), "Langue".into());
                strings.insert("settings_language_desc".into(), "Personnalisez l'interface et le format horaire.".into());
                strings.insert("settings_current_language".into(), "Langue de l'interface".into());
                strings.insert("settings_current_language_desc".into(), "Synchronisée avec le système.".into());
                strings.insert("settings_time_format".into(), "Format horaire".into());
                strings.insert("settings_time_format_desc".into(), "Format utilisé dans l'application.".into());
                // Updates
                strings.insert("settings_section_updates".into(), "Mises à jour".into());
                strings.insert("settings_updates_desc".into(), "Gérez la vérification et le canal des mises à jour.".into());
                strings.insert("settings_auto_check_updates".into(), "Vérifier automatiquement".into());
                strings.insert("settings_auto_check_updates_desc".into(), "Vérifie les nouvelles versions au lancement.".into());
                strings.insert("settings_update_channel".into(), "Canal".into());
                strings.insert("settings_update_channel_desc".into(), "Choisissez la stabilité des versions.".into());
                strings.insert("settings_auto_install_updates".into(), "Installer automatiquement".into());
                strings.insert("settings_auto_install_updates_desc".into(), "Installe les mises à jour en arrière-plan.".into());
                strings.insert("settings_check_updates".into(), "Vérifier les mises à jour".into());
                // Privacy
                strings.insert("settings_section_privacy".into(), "Confidentialité".into());
                strings.insert("settings_privacy_desc".into(), "Choisissez les données partagées avec Colony.".into());
                strings.insert("settings_error_reports".into(), "Envoyer des rapports d'erreurs".into());
                strings.insert("settings_error_reports_desc".into(), "Permet d'améliorer la stabilité.".into());
                strings.insert("settings_usage_stats".into(), "Statistiques anonymes d'utilisation".into());
                strings.insert("settings_usage_stats_desc".into(), "Aide à comprendre l'usage de Colony.".into());
                // Appearance
                strings.insert("settings_appearance_title".into(), "Paramètres d'apparence".into());
                strings.insert("settings_appearance_desc".into(), "Ajustez le thème, les accents et les effets visuels.".into());
                strings.insert("settings_section_theme".into(), "Thème".into());
                strings.insert("settings_theme_desc".into(), "Choisissez le thème de l'interface.".into());
                strings.insert("settings_theme_current".into(), "Thème actuel".into());
                strings.insert("settings_theme_current_desc".into(), "Apparence globale de l'application.".into());
                strings.insert("settings_theme_dark".into(), "Sombre".into());
                // Theme families
                strings.insert("settings_theme_catppuccin".into(), "Catppuccin".into());
                strings.insert("settings_theme_catppuccin_latte".into(), "Latte".into());
                strings.insert("settings_theme_catppuccin_frappe".into(), "Frappé".into());
                strings.insert("settings_theme_catppuccin_macchiato".into(), "Macchiato".into());
                strings.insert("settings_theme_catppuccin_mocha".into(), "Mocha".into());
                strings.insert("settings_theme_gruvbox".into(), "Gruvbox".into());
                strings.insert("settings_theme_light".into(), "Mode clair".into());
                strings.insert("settings_theme_dark_mode".into(), "Mode sombre".into());
                strings.insert("settings_theme_everblush".into(), "Everblush".into());
                strings.insert("settings_theme_kanagawa".into(), "Kanagawa".into());
                strings.insert("settings_theme_kanagawa_journal".into(), "Mode journal".into());
                // New theme families
                strings.insert("settings_theme_nord".into(), "Nord".into());
                strings.insert("settings_theme_dracula".into(), "Dracula".into());
                strings.insert("settings_theme_solarized".into(), "Solarized".into());
                strings.insert("settings_theme_tokyonight".into(), "Tokyo Night".into());
                strings.insert("settings_theme_tokyonight_night".into(), "Nuit".into());
                strings.insert("settings_theme_tokyonight_day".into(), "Jour".into());
                strings.insert("settings_theme_rosepine".into(), "Rosé Pine".into());
                strings.insert("settings_theme_rosepine_main".into(), "Principal".into());
                strings.insert("settings_theme_rosepine_moon".into(), "Lune".into());
                strings.insert("settings_theme_rosepine_dawn".into(), "Aurore".into());
                strings.insert("settings_theme_onedark".into(), "One Dark".into());
                strings.insert("settings_theme_monokai".into(), "Monokai Pro".into());
                strings.insert("settings_theme_monokai_pro".into(), "Pro".into());
                strings.insert("settings_theme_monokai_classic".into(), "Classic".into());
                strings.insert("settings_theme_monokai_spectrum".into(), "Spectrum".into());
                strings.insert("settings_theme_ayu".into(), "Ayu".into());
                strings.insert("settings_theme_ayu_mirage".into(), "Mirage".into());
                strings.insert("settings_theme_everforest".into(), "Everforest".into());
                strings.insert("settings_theme_material".into(), "Material".into());
                strings.insert("settings_theme_material_oceanic".into(), "Oceanic".into());
                strings.insert("settings_theme_material_palenight".into(), "Palenight".into());
                strings.insert("settings_theme_material_deepocean".into(), "Deep Ocean".into());
                strings.insert("settings_theme_flexoki".into(), "Flexoki".into());
                strings.insert("settings_theme_nightfox".into(), "Nightfox".into());
                strings.insert("settings_theme_nightfox_nightfox".into(), "Nightfox".into());
                strings.insert("settings_theme_nightfox_dawnfox".into(), "Dawnfox".into());
                strings.insert("settings_theme_sonokai".into(), "Sonokai".into());
                strings.insert("settings_theme_sonokai_default".into(), "Défaut".into());
                strings.insert("settings_theme_oxocarbon".into(), "Oxocarbon".into());
                strings.insert("settings_theme_nightowl".into(), "Night Owl".into());
                strings.insert("settings_theme_iceberg".into(), "Iceberg".into());
                strings.insert("settings_theme_horizon".into(), "Horizon".into());
                strings.insert("settings_theme_melange".into(), "Mélange".into());
                strings.insert("settings_theme_synthwave".into(), "Synthwave '84".into());
                strings.insert("settings_theme_modus".into(), "Modus".into());
                strings.insert("settings_theme_modus_operandi".into(), "Operandi".into());
                strings.insert("settings_theme_modus_vivendi".into(), "Vivendi".into());
                // Colors & accents
                strings.insert("settings_section_colors".into(), "Couleurs & accents".into());
                strings.insert("settings_colors_desc".into(), "Personnalisez la couleur d'accent de l'interface.".into());
                strings.insert("settings_accent_color".into(), "Couleur d'accent".into());
                strings.insert("settings_accent_color_desc".into(), "Couleur utilisée pour les éléments interactifs.".into());
                strings.insert("settings_accent_red".into(), "Rouge".into());
                strings.insert("settings_accent_orange".into(), "Orange".into());
                strings.insert("settings_accent_yellow".into(), "Jaune".into());
                strings.insert("settings_accent_green".into(), "Vert".into());
                strings.insert("settings_accent_blue".into(), "Bleu".into());
                strings.insert("settings_accent_indigo".into(), "Indigo".into());
                strings.insert("settings_accent_violet".into(), "Violet".into());
                strings.insert("settings_accent_amber".into(), "Ambre".into());
                strings.insert("settings_auto_accent".into(), "Accent automatique selon le fond".into());
                strings.insert("settings_auto_accent_desc".into(), "Adapte automatiquement l'accent aux arrière-plans.".into());
                strings.insert("settings_enabled_label".into(), "Activé".into());
                strings.insert("settings_disabled_label".into(), "Désactivé".into());
                strings.insert("settings_section_typography".into(), "Typographie".into());
                strings.insert("settings_typography_desc".into(), "Configurez la police et la taille du texte.".into());
                strings.insert("settings_font".into(), "Police".into());
                strings.insert("settings_font_desc".into(), "Police utilisée dans l'interface.".into());
                strings.insert("settings_font_size".into(), "Taille du texte".into());
                strings.insert("settings_font_size_desc".into(), "Taille de base du texte.".into());
                strings.insert("settings_font_size_default".into(), "Par défaut".into());
                strings.insert("settings_font_size_small".into(), "Petit".into());
                strings.insert("settings_font_size_large".into(), "Grand".into());
                strings.insert("settings_font_size_xlarge".into(), "Très grand".into());
                strings.insert("settings_section_effects".into(), "Arrière-plans & effets".into());
                strings.insert("settings_effects_desc".into(), "Gérez les animations et effets visuels.".into());
                strings.insert("settings_animations".into(), "Animations".into());
                strings.insert("settings_animations_desc".into(), "Activer les transitions animées.".into());
                strings.insert("settings_section_preview".into(), "Aperçu".into());
                strings.insert("settings_preview_card".into(), "Carte de prévisualisation".into());
                strings.insert("settings_preview_summary".into(), "Thème: Sombre · Accent: Bleu · Texte: Par défaut · Effets: Activés".into());
                // Accessibility
                strings.insert("settings_accessibility_title".into(), "Paramètres d'accessibilité".into());
                strings.insert("settings_accessibility_desc".into(), "Facilitez la lecture, la navigation et la lecture média.".into());
                strings.insert("settings_section_vision".into(), "Vision".into());
                strings.insert("settings_vision_desc".into(), "Options pour améliorer la lisibilité.".into());
                strings.insert("settings_high_contrast".into(), "Contraste élevé".into());
                strings.insert("settings_high_contrast_desc".into(), "Augmente le contraste des éléments.".into());
                strings.insert("settings_disabled".into(), "Désactivé".into());
                strings.insert("settings_text_size_a11y".into(), "Taille du texte".into());
                strings.insert("settings_text_size_a11y_desc".into(), "Ajustez la taille du texte pour le confort.".into());
                strings.insert("settings_section_motion".into(), "Mouvement".into());
                strings.insert("settings_motion_desc".into(), "Réduisez les animations pour le confort.".into());
                strings.insert("settings_reduce_motion".into(), "Réduire les animations".into());
                strings.insert("settings_reduce_motion_desc".into(), "Limite les transitions et mouvements.".into());
                strings.insert("settings_section_navigation".into(), "Navigation & interaction".into());
                strings.insert("settings_navigation_desc".into(), "Options de navigation au clavier et interaction.".into());
                strings.insert("settings_keyboard_nav".into(), "Navigation clavier".into());
                strings.insert("settings_keyboard_nav_desc".into(), "Naviguer avec Tab et les flèches.".into());
                strings.insert("settings_section_reading".into(), "Lecture".into());
                strings.insert("settings_reading_desc".into(), "Options de confort de lecture.".into());
                strings.insert("settings_dyslexia_font".into(), "Police dyslexie".into());
                strings.insert("settings_dyslexia_font_desc".into(), "Utiliser une police adaptée à la dyslexie.".into());
                // Storage
                strings.insert("settings_storage_title".into(), "Stockage".into());
                strings.insert("settings_storage_desc".into(), "Gérez l'emplacement des applications et du cache.".into());
                strings.insert("settings_section_scan".into(), "Scan".into());
                strings.insert("settings_scan_desc".into(), "Configurez les dossiers analysés au démarrage.".into());
                strings.insert("settings_scan_dirs".into(), "Dossiers de scan".into());
                strings.insert("settings_scan_dirs_desc".into(), "Répertoires analysés pour les applications.".into());
                strings.insert("settings_scan_dirs_value".into(), "Par défaut".into());
                strings.insert("settings_startup".into(), "Scanner au démarrage".into());
                strings.insert("settings_startup_desc".into(), "Met à jour la bibliothèque au démarrage.".into());
                strings.insert("settings_enabled".into(), "Activé".into());
                strings.insert("settings_section_install".into(), "Installation".into());
                strings.insert("settings_local_apps".into(), "Applications locales".into());
                strings.insert("settings_colony_repos".into(), "Dépôts Colony".into());
                strings.insert("settings_favorites".into(), "Favoris".into());
                // Placeholders
                strings.insert("settings_coming_soon".into(), "Bientôt".into());
                // About
                strings.insert("settings_about_title".into(), "À propos de Colony".into());
                strings.insert("settings_about".into(), "À propos".into());
                strings.insert("settings_version".into(), "Colony v0.1.0".into());
                // Launcher self-update
                strings.insert("launcher_update_available".into(), "Colony {version} est disponible !".into());
                strings.insert("launcher_update_available_short".into(), "\u{f0aa}  Mise à jour {version}".into());
                strings.insert("launcher_update_ready".into(), "Mise à jour prête. Cliquez pour relancer Colony.".into());
                strings.insert("launcher_restart_to_update".into(), "\u{f021}  Relancer pour mettre à jour".into());
                strings.insert("launcher_download_update".into(), "Télécharger la mise à jour {version}".into());
                strings.insert("launcher_update_failed".into(), "Échec de la mise à jour : {error}".into());
                strings.insert("check_launcher_updates".into(), "Vérifier les mises à jour".into());
                strings.insert("launcher_up_to_date".into(), "Colony est à jour".into());
                // Detail tabs
                strings.insert("tab_readme".into(), "ReadMe".into());
                strings.insert("tab_license".into(), "License".into());
                strings.insert("tab_changelog".into(), "Changelog".into());
                strings.insert("tab_loading".into(), "Chargement...".into());
                strings.insert("tab_not_available".into(), "Non disponible".into());
            }
            _ => {
                // English (default)
                // Sidebar
                strings.insert("categories".into(), "Categories".into());
                strings.insert("rescan".into(), "Rescan".into());

                // GitHub panel
                strings.insert("github_connect_desc".into(), "Connect to GitHub to detect Colony repos (colony.json) from the MotherSphere account.".into());
                strings.insert("github_login".into(), "Sign in with GitHub".into());
                strings.insert("github_public_api".into(), "Not connected: Public GitHub API (60 req/h)".into());
                strings.insert("github_rate_limit".into(), "GitHub rate limit reached. Retry in {wait} seconds.".into());
                strings.insert("github_enter_code".into(), "Enter this code on GitHub:".into());
                strings.insert("github_copy_hint".into(), "Click to copy — Waiting for authorization...".into());
                strings.insert("github_connecting".into(), "Connecting...".into());
                strings.insert("github_connected".into(), "Connected".into());
                strings.insert("github_repos_detected".into(), "{count} Colony repos detected".into());
                strings.insert("github_no_repos".into(), "No repos with colony.json found.".into());
                strings.insert("github_refresh".into(), "Refresh repos".into());
                strings.insert("github_logout".into(), "Sign out".into());
                strings.insert("github_error".into(), "Error: {error}".into());
                strings.insert("github_retry".into(), "Retry".into());
                strings.insert("github_disconnected".into(), "Disconnected from GitHub".into());

                // App grid
                strings.insert("no_apps_found".into(), "No applications found".into());
                strings.insert("search_placeholder".into(), "Search applications...".into());

                // Detail view
                strings.insert("back".into(), "Back".into());
                strings.insert("language_label".into(), "Language: {lang}".into());
                strings.insert("launch".into(), "Launch {name}".into());
                strings.insert("update".into(), "Update".into());
                strings.insert("download".into(), "Download".into());
                strings.insert("no_release".into(), "No release available".into());
                strings.insert("no_release_platform".into(), "Not available for your platform".into());

                // Status messages
                strings.insert("apps_found".into(), "{count} applications found".into());
                strings.insert("app_launched".into(), "Application launched.".into());
                strings.insert("installed".into(), "Installed: {path}".into());
                strings.insert("download_error".into(), "Download error: {error}".into());
                strings.insert("downloading".into(), "Downloading {file}…".into());
                strings.insert("no_release_for".into(), "No release for {platform}".into());
                strings.insert("uninstalled".into(), "{name} uninstalled.".into());
                strings.insert("launch_error".into(), "Cannot launch: {error}".into());
                strings.insert("launch_error_empty".into(), "Cannot launch: empty command".into());
                strings.insert("uninstall_error".into(), "Uninstall error: {error}".into());

                // OAuth errors
                strings.insert("oauth_error".into(), "OAuth error: {error}".into());
                strings.insert("github_api_error".into(), "GitHub error: {error}".into());
                strings.insert("scan_error".into(), "Error: {error}".into());
                strings.insert("launch_error_msg".into(), "Launch error: {error}".into());
                strings.insert("updates_available".into(), "{count} update(s) available: {names}".into());

                // Thread errors
                strings.insert("error_thread_panic".into(), "Internal error: background thread panicked".into());

                // Download cancellation
                strings.insert("download_cancelled".into(), "Download cancelled".into());

                // Uninstall confirmation
                strings.insert("confirm_uninstall".into(), "Are you sure you want to uninstall \"{name}\"? This action cannot be undone.".into());
                strings.insert("cancel".into(), "Cancel".into());
                strings.insert("confirm_delete".into(), "Uninstall".into());

                // Favorites
                strings.insert("add_favorite".into(), "Add to favorites".into());
                strings.insert("remove_favorite".into(), "Remove from favorites".into());

                // First launch — carousel (3 steps)
                strings.insert("welcome_title".into(), "Welcome to Colony".into());
                strings.insert("welcome_desc".into(), "The centralized launcher for the MotherSphere ecosystem. Discover, install and launch apps in one click.".into());
                // Step 1 — interface tour
                strings.insert("welcome_step1_title".into(), "The interface, in 3 zones".into());
                strings.insert("welcome_step1_tip1_title".into(), "Sidebar".into());
                strings.insert("welcome_step1_tip1_desc".into(), "Filter by category or origin (Colony / system apps).".into());
                strings.insert("welcome_step1_tip2_title".into(), "Search".into());
                strings.insert("welcome_step1_tip2_desc".into(), "Type an app name in the top bar to filter instantly.".into());
                strings.insert("welcome_step1_tip3_title".into(), "Detail".into());
                strings.insert("welcome_step1_tip3_desc".into(), "Click any app to read its README, changelog and install it.".into());
                // Step 2 — GitHub + ready
                strings.insert("welcome_step2_title".into(), "Connect GitHub (optional)".into());
                strings.insert("welcome_step2_desc".into(), "Without an account: 60 GitHub requests per hour. With an account: 5000/h + access to your private repos. Recommended if you plan to browse a lot.".into());
                strings.insert("welcome_step2_hint1".into(), "\u{f005}  Favorites (⭐) for quick access".into());
                strings.insert("welcome_step2_hint2".into(), "\u{f53f}  24 theme families in the preferences".into());
                strings.insert("welcome_step2_hint3".into(), "\u{f059}  Full tutorial + FAQ on GitHub".into());
                // Navigation
                strings.insert("welcome_start".into(), "Let's go!".into());
                strings.insert("welcome_next".into(), "Next".into());
                strings.insert("welcome_back".into(), "Back".into());
                strings.insert("welcome_skip".into(), "Skip".into());
                strings.insert("welcome_connect_now".into(), "Connect now".into());
                strings.insert("welcome_later".into(), "Later".into());

                // Loading / async feedback
                strings.insert("loading".into(), "Loading...".into());
                strings.insert("scanning".into(), "Scanning...".into());
                strings.insert("checking_updates".into(), "Checking for updates...".into());
                strings.insert("syncing_repos".into(), "Syncing repositories...".into());
                strings.insert("no_results_for".into(), "No results for \"{query}\"".into());
                strings.insert("n_results_found".into(), "{count} result(s) for \"{query}\"".into());
                strings.insert("language_restart_notice".into(), "Language change will take effect on next launch.".into());
                strings.insert("theme_applied".into(), "Theme applied.".into());

                // Keyboard shortcuts
                strings.insert("shortcuts_title".into(), "Keyboard shortcuts".into());
                strings.insert("shortcut_esc".into(), "Esc — Close active panel".into());
                strings.insert("shortcut_tab".into(), "Tab / Shift+Tab — Navigate categories".into());
                strings.insert("shortcut_arrows".into(), "↑ ↓ — Navigate settings".into());
                strings.insert("shortcut_enter".into(), "Enter — Open first visible item".into());
                strings.insert("shortcut_pageupdown".into(), "Page Up/Down — Fast navigation in settings".into());

                // Tooltips / hints
                strings.insert("hint_settings".into(), "Open preferences".into());
                strings.insert("hint_search".into(), "Type to filter applications".into());
                strings.insert("hint_favorites".into(), "Click the star to add to favorites".into());
                strings.insert("hint_keyboard".into(), "Use Tab and arrow keys to navigate".into());

                // Settings
                strings.insert("settings_title".into(), "Preferences".into());
                strings.insert("settings_close".into(), "Close".into());
                strings.insert("settings_cat_general".into(), "General".into());
                strings.insert("settings_cat_appearance".into(), "Appearance".into());
                strings.insert("settings_cat_accessibility".into(), "Accessibility".into());
                strings.insert("settings_cat_storage".into(), "Storage".into());
                strings.insert("settings_cat_about".into(), "About".into());
                strings.insert("settings_cat_shortcuts".into(), "Shortcuts".into());
                // General
                strings.insert("settings_general_title".into(), "General settings".into());
                strings.insert("settings_general_desc".into(), "Preferences are saved automatically.".into());
                // Startup
                strings.insert("settings_section_startup".into(), "Startup".into());
                strings.insert("settings_startup_section_desc".into(), "Manage Colony startup and session restoration.".into());
                strings.insert("settings_auto_scan".into(), "Scan on startup".into());
                strings.insert("settings_auto_scan_desc".into(), "Scan directories when Colony opens.".into());
                strings.insert("settings_restore_session".into(), "Restore last session".into());
                strings.insert("settings_restore_session_desc".into(), "Category and screen from last usage.".into());
                strings.insert("settings_default_view".into(), "Open on".into());
                strings.insert("settings_default_view_desc".into(), "Choose the default screen.".into());
                strings.insert("settings_default_view_all".into(), "All".into());
                strings.insert("settings_default_view_favorites".into(), "Favorites".into());
                strings.insert("settings_default_view_recent".into(), "Recent".into());
                strings.insert("settings_close_behavior".into(), "Close behavior".into());
                strings.insert("settings_close_behavior_desc".into(), "Choose action on close.".into());
                strings.insert("settings_close_quit".into(), "Quit".into());
                strings.insert("settings_close_tray".into(), "Minimize to tray".into());
                // Language
                strings.insert("settings_section_language".into(), "Language".into());
                strings.insert("settings_language_desc".into(), "Customize the interface and time format.".into());
                strings.insert("settings_current_language".into(), "Interface language".into());
                strings.insert("settings_current_language_desc".into(), "Synced with system.".into());
                strings.insert("settings_time_format".into(), "Time format".into());
                strings.insert("settings_time_format_desc".into(), "Format used in the application.".into());
                // Updates
                strings.insert("settings_section_updates".into(), "Updates".into());
                strings.insert("settings_updates_desc".into(), "Manage update checking and channel.".into());
                strings.insert("settings_auto_check_updates".into(), "Check automatically".into());
                strings.insert("settings_auto_check_updates_desc".into(), "Check for new versions on launch.".into());
                strings.insert("settings_update_channel".into(), "Channel".into());
                strings.insert("settings_update_channel_desc".into(), "Choose version stability.".into());
                strings.insert("settings_auto_install_updates".into(), "Install automatically".into());
                strings.insert("settings_auto_install_updates_desc".into(), "Install updates in background.".into());
                strings.insert("settings_check_updates".into(), "Check for updates".into());
                // Privacy
                strings.insert("settings_section_privacy".into(), "Privacy".into());
                strings.insert("settings_privacy_desc".into(), "Choose data shared with Colony.".into());
                strings.insert("settings_error_reports".into(), "Send error reports".into());
                strings.insert("settings_error_reports_desc".into(), "Helps improve stability.".into());
                strings.insert("settings_usage_stats".into(), "Anonymous usage statistics".into());
                strings.insert("settings_usage_stats_desc".into(), "Helps understand Colony usage.".into());
                // Appearance
                strings.insert("settings_appearance_title".into(), "Appearance settings".into());
                strings.insert("settings_appearance_desc".into(), "Adjust theme, accents and visual effects.".into());
                strings.insert("settings_section_theme".into(), "Theme".into());
                strings.insert("settings_theme_desc".into(), "Choose the interface theme.".into());
                strings.insert("settings_theme_current".into(), "Current theme".into());
                strings.insert("settings_theme_current_desc".into(), "Overall application appearance.".into());
                strings.insert("settings_theme_dark".into(), "Dark".into());
                // Theme families
                strings.insert("settings_theme_catppuccin".into(), "Catppuccin".into());
                strings.insert("settings_theme_catppuccin_latte".into(), "Latte".into());
                strings.insert("settings_theme_catppuccin_frappe".into(), "Frappé".into());
                strings.insert("settings_theme_catppuccin_macchiato".into(), "Macchiato".into());
                strings.insert("settings_theme_catppuccin_mocha".into(), "Mocha".into());
                strings.insert("settings_theme_gruvbox".into(), "Gruvbox".into());
                strings.insert("settings_theme_light".into(), "Light mode".into());
                strings.insert("settings_theme_dark_mode".into(), "Dark mode".into());
                strings.insert("settings_theme_everblush".into(), "Everblush".into());
                strings.insert("settings_theme_kanagawa".into(), "Kanagawa".into());
                strings.insert("settings_theme_kanagawa_journal".into(), "Journal mode".into());
                // New theme families
                strings.insert("settings_theme_nord".into(), "Nord".into());
                strings.insert("settings_theme_dracula".into(), "Dracula".into());
                strings.insert("settings_theme_solarized".into(), "Solarized".into());
                strings.insert("settings_theme_tokyonight".into(), "Tokyo Night".into());
                strings.insert("settings_theme_tokyonight_night".into(), "Night".into());
                strings.insert("settings_theme_tokyonight_day".into(), "Day".into());
                strings.insert("settings_theme_rosepine".into(), "Rosé Pine".into());
                strings.insert("settings_theme_rosepine_main".into(), "Main".into());
                strings.insert("settings_theme_rosepine_moon".into(), "Moon".into());
                strings.insert("settings_theme_rosepine_dawn".into(), "Dawn".into());
                strings.insert("settings_theme_onedark".into(), "One Dark".into());
                strings.insert("settings_theme_monokai".into(), "Monokai Pro".into());
                strings.insert("settings_theme_monokai_pro".into(), "Pro".into());
                strings.insert("settings_theme_monokai_classic".into(), "Classic".into());
                strings.insert("settings_theme_monokai_spectrum".into(), "Spectrum".into());
                strings.insert("settings_theme_ayu".into(), "Ayu".into());
                strings.insert("settings_theme_ayu_mirage".into(), "Mirage".into());
                strings.insert("settings_theme_everforest".into(), "Everforest".into());
                strings.insert("settings_theme_material".into(), "Material".into());
                strings.insert("settings_theme_material_oceanic".into(), "Oceanic".into());
                strings.insert("settings_theme_material_palenight".into(), "Palenight".into());
                strings.insert("settings_theme_material_deepocean".into(), "Deep Ocean".into());
                strings.insert("settings_theme_flexoki".into(), "Flexoki".into());
                strings.insert("settings_theme_nightfox".into(), "Nightfox".into());
                strings.insert("settings_theme_nightfox_nightfox".into(), "Nightfox".into());
                strings.insert("settings_theme_nightfox_dawnfox".into(), "Dawnfox".into());
                strings.insert("settings_theme_sonokai".into(), "Sonokai".into());
                strings.insert("settings_theme_sonokai_default".into(), "Default".into());
                strings.insert("settings_theme_oxocarbon".into(), "Oxocarbon".into());
                strings.insert("settings_theme_nightowl".into(), "Night Owl".into());
                strings.insert("settings_theme_iceberg".into(), "Iceberg".into());
                strings.insert("settings_theme_horizon".into(), "Horizon".into());
                strings.insert("settings_theme_melange".into(), "Melange".into());
                strings.insert("settings_theme_synthwave".into(), "Synthwave '84".into());
                strings.insert("settings_theme_modus".into(), "Modus".into());
                strings.insert("settings_theme_modus_operandi".into(), "Operandi".into());
                strings.insert("settings_theme_modus_vivendi".into(), "Vivendi".into());
                // Colors & accents
                strings.insert("settings_section_colors".into(), "Colors & accents".into());
                strings.insert("settings_colors_desc".into(), "Customize the interface accent color.".into());
                strings.insert("settings_accent_color".into(), "Accent color".into());
                strings.insert("settings_accent_color_desc".into(), "Color used for interactive elements.".into());
                strings.insert("settings_accent_red".into(), "Red".into());
                strings.insert("settings_accent_orange".into(), "Orange".into());
                strings.insert("settings_accent_yellow".into(), "Yellow".into());
                strings.insert("settings_accent_green".into(), "Green".into());
                strings.insert("settings_accent_blue".into(), "Blue".into());
                strings.insert("settings_accent_indigo".into(), "Indigo".into());
                strings.insert("settings_accent_violet".into(), "Violet".into());
                strings.insert("settings_accent_amber".into(), "Amber".into());
                strings.insert("settings_auto_accent".into(), "Auto accent from background".into());
                strings.insert("settings_auto_accent_desc".into(), "Automatically adapts accent to backgrounds.".into());
                strings.insert("settings_enabled_label".into(), "Enabled".into());
                strings.insert("settings_disabled_label".into(), "Disabled".into());
                strings.insert("settings_section_typography".into(), "Typography".into());
                strings.insert("settings_typography_desc".into(), "Configure font and text size.".into());
                strings.insert("settings_font".into(), "Font".into());
                strings.insert("settings_font_desc".into(), "Font used in the interface.".into());
                strings.insert("settings_font_size".into(), "Text size".into());
                strings.insert("settings_font_size_desc".into(), "Base text size.".into());
                strings.insert("settings_font_size_default".into(), "Default".into());
                strings.insert("settings_font_size_small".into(), "Small".into());
                strings.insert("settings_font_size_large".into(), "Large".into());
                strings.insert("settings_font_size_xlarge".into(), "Extra large".into());
                strings.insert("settings_section_effects".into(), "Backgrounds & effects".into());
                strings.insert("settings_effects_desc".into(), "Manage animations and visual effects.".into());
                strings.insert("settings_animations".into(), "Animations".into());
                strings.insert("settings_animations_desc".into(), "Enable animated transitions.".into());
                strings.insert("settings_section_preview".into(), "Preview".into());
                strings.insert("settings_preview_card".into(), "Preview card".into());
                strings.insert("settings_preview_summary".into(), "Theme: Dark · Accent: Blue · Text: Default · Effects: Enabled".into());
                // Accessibility
                strings.insert("settings_accessibility_title".into(), "Accessibility settings".into());
                strings.insert("settings_accessibility_desc".into(), "Improve reading, navigation and media playback.".into());
                strings.insert("settings_section_vision".into(), "Vision".into());
                strings.insert("settings_vision_desc".into(), "Options to improve readability.".into());
                strings.insert("settings_high_contrast".into(), "High contrast".into());
                strings.insert("settings_high_contrast_desc".into(), "Increase contrast of elements.".into());
                strings.insert("settings_disabled".into(), "Disabled".into());
                strings.insert("settings_text_size_a11y".into(), "Text size".into());
                strings.insert("settings_text_size_a11y_desc".into(), "Adjust text size for comfort.".into());
                strings.insert("settings_section_motion".into(), "Motion".into());
                strings.insert("settings_motion_desc".into(), "Reduce animations for comfort.".into());
                strings.insert("settings_reduce_motion".into(), "Reduce motion".into());
                strings.insert("settings_reduce_motion_desc".into(), "Limit transitions and movements.".into());
                strings.insert("settings_section_navigation".into(), "Navigation & interaction".into());
                strings.insert("settings_navigation_desc".into(), "Keyboard navigation and interaction options.".into());
                strings.insert("settings_keyboard_nav".into(), "Keyboard navigation".into());
                strings.insert("settings_keyboard_nav_desc".into(), "Navigate with Tab and arrow keys.".into());
                strings.insert("settings_section_reading".into(), "Reading".into());
                strings.insert("settings_reading_desc".into(), "Reading comfort options.".into());
                strings.insert("settings_dyslexia_font".into(), "Dyslexia font".into());
                strings.insert("settings_dyslexia_font_desc".into(), "Use a font adapted for dyslexia.".into());
                // Storage
                strings.insert("settings_storage_title".into(), "Storage".into());
                strings.insert("settings_storage_desc".into(), "Manage application locations and cache.".into());
                strings.insert("settings_section_scan".into(), "Scan".into());
                strings.insert("settings_scan_desc".into(), "Configure directories scanned at startup.".into());
                strings.insert("settings_scan_dirs".into(), "Scan directories".into());
                strings.insert("settings_scan_dirs_desc".into(), "Directories scanned for applications.".into());
                strings.insert("settings_scan_dirs_value".into(), "Default".into());
                strings.insert("settings_startup".into(), "Scan on startup".into());
                strings.insert("settings_startup_desc".into(), "Updates the library at startup.".into());
                strings.insert("settings_enabled".into(), "Enabled".into());
                strings.insert("settings_section_install".into(), "Installation".into());
                strings.insert("settings_local_apps".into(), "Local applications".into());
                strings.insert("settings_colony_repos".into(), "Colony repos".into());
                strings.insert("settings_favorites".into(), "Favorites".into());
                // Placeholders
                strings.insert("settings_coming_soon".into(), "Coming soon".into());
                // About
                strings.insert("settings_about_title".into(), "About Colony".into());
                strings.insert("settings_about".into(), "About".into());
                strings.insert("settings_version".into(), "Colony v0.1.0".into());
                // Launcher self-update
                strings.insert("launcher_update_available".into(), "Colony {version} is available!".into());
                strings.insert("launcher_update_available_short".into(), "\u{f0aa}  Update {version}".into());
                strings.insert("launcher_update_ready".into(), "Update ready. Click to restart Colony.".into());
                strings.insert("launcher_restart_to_update".into(), "\u{f021}  Restart to update".into());
                strings.insert("launcher_download_update".into(), "Download update {version}".into());
                strings.insert("launcher_update_failed".into(), "Update failed: {error}".into());
                strings.insert("check_launcher_updates".into(), "Check for updates".into());
                strings.insert("launcher_up_to_date".into(), "Colony is up to date".into());
                // Detail tabs
                strings.insert("tab_readme".into(), "ReadMe".into());
                strings.insert("tab_license".into(), "License".into());
                strings.insert("tab_changelog".into(), "Changelog".into());
                strings.insert("tab_loading".into(), "Loading...".into());
                strings.insert("tab_not_available".into(), "Not available".into());
            }
        }

        Self {
            strings,
            lang: lang.to_string(),
        }
    }
}

/// Initialize the locale system. Call once at startup.
pub fn init() {
    let lang = detect_language();
    tracing::info!("Locale: {lang}");
    LOCALE.get_or_init(|| Locale::new(&lang));
}

/// Get a translated string by key.
pub fn t(key: &str) -> String {
    LOCALE
        .get()
        .and_then(|locale| locale.strings.get(key))
        .cloned()
        .unwrap_or_else(|| {
            tracing::warn!("Missing translation key: {key}");
            key.to_string()
        })
}

/// Get a translated string with variable substitution.
/// Variables use `{name}` syntax.
pub fn t_fmt(key: &str, vars: &[(&str, &str)]) -> String {
    let mut result = t(key);
    for (name, value) in vars {
        result = result.replace(&format!("{{{name}}}"), value);
    }
    result
}

/// Get the current language code.
pub fn current_lang() -> &'static str {
    LOCALE
        .get()
        .map(|l| l.lang.as_str())
        .unwrap_or("en")
}

/// Detect the user's language from environment.
fn detect_language() -> String {
    // Check LANG, LC_ALL, LC_MESSAGES
    for var in ["LC_ALL", "LC_MESSAGES", "LANG"] {
        if let Ok(val) = std::env::var(var) {
            let lang = val.split('.').next().unwrap_or(&val);
            let lang = lang.split('_').next().unwrap_or(lang);
            if lang == "fr" {
                return "fr".to_string();
            }
        }
    }

    "en".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn english_locale_has_keys() {
        let locale = Locale::new("en");
        assert!(locale.strings.contains_key("categories"));
        assert!(locale.strings.contains_key("github_login"));
        assert!(locale.strings.contains_key("back"));
        assert!(locale.strings.contains_key("error_thread_panic"));
        assert!(locale.strings.contains_key("confirm_uninstall"));
        assert!(locale.strings.contains_key("welcome_title"));
        assert!(locale.strings.contains_key("download_cancelled"));
        assert!(locale.strings.contains_key("add_favorite"));
    }

    #[test]
    fn french_locale_has_keys() {
        let locale = Locale::new("fr");
        assert_eq!(locale.strings.get("categories").unwrap(), "Catégories");
        assert_eq!(locale.strings.get("back").unwrap(), "Retour");
        assert!(locale.strings.contains_key("error_thread_panic"));
        assert!(locale.strings.contains_key("confirm_uninstall"));
        assert!(locale.strings.contains_key("welcome_title"));
    }

    #[test]
    fn unknown_lang_defaults_to_english() {
        let locale = Locale::new("xx");
        assert_eq!(locale.strings.get("categories").unwrap(), "Categories");
    }

    #[test]
    fn t_fmt_substitution() {
        // Initialize with English for test
        let _ = LOCALE.set(Locale::new("en"));
        let result = t_fmt("apps_found", &[("count", "42")]);
        assert_eq!(result, "42 applications found");
    }
}
