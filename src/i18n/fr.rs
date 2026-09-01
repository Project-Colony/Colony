//! French strings.
//!
//! One file per locale so a new string is a one-locale diff. The key sets of the
//! two locales must match exactly - `super::tests::fr_and_en_have_identical_key_sets`
//! fails otherwise.

use std::collections::HashMap;

pub(super) fn insert_all(strings: &mut HashMap<String, String>) {
    // Sidebar
    strings.insert("categories".into(), "Catégories".into());
    strings.insert("rescan".into(), "Rescan".into());

    // GitHub panel
    strings.insert("github_connect_desc".into(), "Connectez-vous à GitHub pour détecter les dépôts Colony (colony.json) de l'organisation Project-Colony.".into());
    strings.insert("github_login".into(), "Se connecter avec GitHub".into());
    strings.insert(
        "github_public_api".into(),
        "Mode non connecté : API publique GitHub (60 req/h)".into(),
    );
    strings.insert(
        "github_rate_limit".into(),
        "Quota GitHub atteint. Réessayez dans {wait} secondes.".into(),
    );
    strings.insert(
        "github_enter_code".into(),
        "Entrez ce code sur GitHub :".into(),
    );
    strings.insert(
        "github_copy_hint".into(),
        "Cliquez pour copier — En attente d'autorisation...".into(),
    );
    strings.insert("github_connecting".into(), "Connexion en cours...".into());
    strings.insert("github_connected".into(), "Connecté".into());
    strings.insert(
        "github_repos_detected".into(),
        "{count} dépôts Colony détectés".into(),
    );
    strings.insert(
        "github_no_repos".into(),
        "Aucun dépôt avec colony.json trouvé.".into(),
    );
    strings.insert("github_refresh".into(), "Rafraîchir les dépôts".into());
    strings.insert("github_logout".into(), "Se déconnecter".into());
    strings.insert("github_error".into(), "Erreur : {error}".into());
    strings.insert("github_retry".into(), "Réessayer".into());
    strings.insert("github_disconnected".into(), "Déconnecté de GitHub".into());

    // App grid
    strings.insert("no_apps_found".into(), "Aucune application trouvée".into());
    strings.insert(
        "search_placeholder".into(),
        "Rechercher des applications...".into(),
    );
    strings.insert("status_installed".into(), "Installé".into());
    strings.insert("status_get".into(), "À installer".into());
    strings.insert("status_unavailable".into(), "Indisponible".into());
    strings.insert("status_update".into(), "Mise à jour".into());

    // Detail view
    strings.insert("back".into(), "Retour".into());
    strings.insert("language_label".into(), "Langage: {lang}".into());
    strings.insert("launch".into(), "Lancer {name}".into());
    strings.insert("update".into(), "Mettre à jour".into());
    strings.insert("download".into(), "Télécharger".into());
    strings.insert("offered_version".into(), "Version {version}".into());
    strings.insert(
        "no_release_unrecognized".into(),
        "Aucune version installable - cette application ne publie pas d'assets reconnus par Colony"
            .into(),
    );
    strings.insert(
        "no_release_platform".into(),
        "Non disponible pour votre plateforme".into(),
    );

    // Status messages
    strings.insert("apps_found".into(), "{count} applications trouvées".into());
    strings.insert("app_launched".into(), "Application lancée.".into());
    strings.insert("installed".into(), "Installé : {path}".into());
    strings.insert(
        "download_error".into(),
        "Erreur téléchargement : {error}".into(),
    );
    strings.insert("downloading".into(), "Téléchargement de {file}…".into());
    strings.insert(
        "no_release_for".into(),
        "Pas de release pour {platform}".into(),
    );
    strings.insert("uninstalled".into(), "{name} désinstallé.".into());
    strings.insert(
        "launch_error".into(),
        "Impossible de lancer: {error}".into(),
    );
    strings.insert(
        "launch_error_empty".into(),
        "Impossible de lancer: commande vide".into(),
    );
    strings.insert(
        "uninstall_error".into(),
        "Erreur désinstallation : {error}".into(),
    );

    // OAuth errors
    strings.insert("oauth_error".into(), "Erreur OAuth: {error}".into());
    strings.insert(
        "oauth_device_expired".into(),
        "Délai dépassé : l'autorisation GitHub n'a pas été confirmée à temps.".into(),
    );
    strings.insert(
        "oauth_device_failed".into(),
        "Échec de la connexion GitHub : {error} — {desc}".into(),
    );
    strings.insert("github_api_error".into(), "Erreur GitHub: {error}".into());
    strings.insert("scan_error".into(), "Erreur: {error}".into());
    strings.insert(
        "launch_error_msg".into(),
        "Erreur lancement : {error}".into(),
    );
    strings.insert(
        "updates_available".into(),
        "{count} mise(s) à jour disponible(s) : {names}".into(),
    );
    strings.insert(
        "launcher_relaunch_failed".into(),
        "Colony a été mis à jour mais la nouvelle version n'a pas démarré ({error}). L'ancienne version est conservée dans {backup} - renommez-la pour revenir en arrière.".into(),
    );
    strings.insert(
        "logout_incomplete".into(),
        "Déconnecté, mais le jeton stocké n'a pas pu être supprimé ({error}). Révoquez-le sur github.com/settings/applications.".into(),
    );
    strings.insert(
        "update_skipped".into(),
        "{name} ignorée : absente du catalogue, ou aucune version pour cette plateforme".into(),
    );
    strings.insert(
        "update_check_failed".into(),
        "Impossible de vérifier {count} application(s) — elles ne sont peut-être pas à jour".into(),
    );

    // Sidebar section names (localized)
    strings.insert("section_all".into(), "Tout".into());
    strings.insert("section_favorites".into(), "Favoris".into());
    strings.insert("section_windows".into(), "Windows".into());
    strings.insert("section_linux".into(), "Linux".into());
    strings.insert("section_macos".into(), "macOS".into());
    strings.insert("section_development".into(), "Développement".into());
    strings.insert("section_graphics".into(), "Graphisme".into());
    strings.insert("section_network".into(), "Réseau".into());
    strings.insert("section_office".into(), "Bureautique".into());
    strings.insert("section_multimedia".into(), "Multimédia".into());
    strings.insert("section_system".into(), "Système".into());
    strings.insert("section_utilities".into(), "Utilitaires".into());
    strings.insert("section_games".into(), "Jeux".into());
    strings.insert("section_other".into(), "Autre".into());

    // Thread errors
    strings.insert(
        "error_thread_panic".into(),
        "Erreur interne : le thread a paniqué".into(),
    );

    // Download cancellation
    strings.insert("download_cancelled".into(), "Téléchargement annulé".into());

    // Uninstall confirmation
    strings.insert(
        "confirm_uninstall".into(),
        "Voulez-vous vraiment désinstaller « {name} » ? Cette action est irréversible.".into(),
    );
    strings.insert("cancel".into(), "Annuler".into());
    strings.insert("confirm_delete".into(), "Désinstaller".into());

    // Favorites
    strings.insert("add_favorite".into(), "Ajouter aux favoris".into());

    // First launch — carousel (3 steps)
    strings.insert("welcome_title".into(), "Bienvenue dans Colony".into());
    strings.insert("welcome_desc".into(), "Le lanceur centralisé de l'écosystème Project-Colony. Découvrez, installez et lancez vos apps en un clic.".into());
    // Step 1 — interface tour
    // Step 2 — GitHub + ready
    // Navigation
    strings.insert("welcome_start".into(), "C'est parti !".into());
    strings.insert("welcome_next".into(), "Suivant".into());
    strings.insert("welcome_back".into(), "Retour".into());
    strings.insert("welcome_skip".into(), "Passer".into());
    strings.insert("welcome_connect_now".into(), "Connecter maintenant".into());
    strings.insert("welcome_later".into(), "Plus tard".into());

    // Tutoriel guidé (spotlight sur l'UI réelle)
    strings.insert("tut_sidebar_title".into(), "Les catégories".into());
    strings.insert("tut_sidebar_desc".into(), "Filtrez vos apps par type : jeux, outils, favoris, ou par origine (écosystème Colony vs. système). La barre latérale reste toujours visible.".into());
    strings.insert("tut_search_title".into(), "La recherche".into());
    strings.insert("tut_search_desc".into(), "Tapez le nom d'une app pour la retrouver instantanément, peu importe la catégorie sélectionnée.".into());
    strings.insert("tut_grid_title".into(), "Vos applications".into());
    strings.insert("tut_grid_desc".into(), "Voici toutes vos apps installées et les apps Colony disponibles. Cliquez une carte pour voir son README, son changelog et l'installer en un clic.".into());
    strings.insert(
        "tut_github_title".into(),
        "Connexion GitHub (optionnel)".into(),
    );
    strings.insert("tut_github_desc".into(), "Sans compte : 60 requêtes/h. Avec compte : 5000/h + accès aux repos privés. Recommandé si vous explorez beaucoup. Le bouton Rescan juste en dessous relance l'analyse système.".into());
    strings.insert("tut_finish_title".into(), "Vous êtes prêt !".into());
    strings.insert("tut_finish_desc".into(), "L'icône d'engrenage à côté du titre ouvre les préférences : 24 familles de thèmes, raccourcis clavier, accessibilité. Bon voyage dans Colony !".into());

    // Loading / async feedback
    strings.insert("scanning".into(), "Analyse en cours...".into());
    strings.insert(
        "checking_updates".into(),
        "Vérification des mises à jour...".into(),
    );
    strings.insert(
        "syncing_repos".into(),
        "Synchronisation des dépôts...".into(),
    );
    strings.insert(
        "no_results_for".into(),
        "Aucun résultat pour « {query} »".into(),
    );
    strings.insert(
        "n_results_found".into(),
        "{count} résultat(s) pour « {query} »".into(),
    );
    strings.insert("theme_applied".into(), "Thème appliqué.".into());

    // Keyboard shortcuts
    strings.insert("shortcuts_title".into(), "Raccourcis clavier".into());
    strings.insert(
        "shortcut_esc".into(),
        "Échap — Fermer le panneau, la boîte de dialogue ou la fiche en cours".into(),
    );
    strings.insert(
        "shortcut_tab".into(),
        "Tab / Maj+Tab — Naviguer entre les catégories".into(),
    );
    strings.insert(
        "shortcut_arrows".into(),
        "↑ ↓ — Naviguer dans les réglages et la grille".into(),
    );
    strings.insert(
        "shortcut_enter".into(),
        "Entrée — Ouvrir l'élément sélectionné (lance directement une app locale)".into(),
    );
    strings.insert(
        "shortcut_pageupdown".into(),
        "Page ↑/↓ — Naviguer plus vite dans les paramètres".into(),
    );

    // Tooltips / hints
    strings.insert("hint_settings".into(), "Ouvrir les préférences".into());
    strings.insert(
        "hint_search".into(),
        "Tapez pour filtrer les applications".into(),
    );
    strings.insert(
        "hint_favorites".into(),
        "Cliquez sur l'étoile pour ajouter aux favoris".into(),
    );
    strings.insert(
        "hint_keyboard".into(),
        "Utilisez Tab et les flèches pour naviguer".into(),
    );

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
    strings.insert(
        "settings_general_title".into(),
        "Paramètres généraux".into(),
    );
    strings.insert(
        "settings_general_desc".into(),
        "Les préférences sont enregistrées automatiquement.".into(),
    );
    // Startup
    strings.insert("settings_section_startup".into(), "Démarrage".into());
    strings.insert(
        "settings_startup_section_desc".into(),
        "Gérez l'ouverture de Colony et la restauration des sessions.".into(),
    );
    strings.insert(
        "settings_restore_session".into(),
        "Restaurer la dernière session".into(),
    );
    strings.insert(
        "settings_restore_session_desc".into(),
        "Catégorie et écran affichés au dernier usage.".into(),
    );
    strings.insert("settings_default_view".into(), "Ouvrir sur".into());
    strings.insert(
        "settings_default_view_desc".into(),
        "Choisissez l'écran par défaut.".into(),
    );
    strings.insert("settings_default_view_all".into(), "Toutes".into());
    strings.insert("settings_default_view_favorites".into(), "Favoris".into());
    // Language
    strings.insert("settings_section_language".into(), "Langue".into());
    strings.insert(
        "settings_language_desc".into(),
        "Personnalisez l'interface et le format horaire.".into(),
    );
    strings.insert(
        "settings_current_language".into(),
        "Langue de l'interface".into(),
    );
    strings.insert(
        "settings_current_language_desc".into(),
        "Synchronisée avec le système.".into(),
    );
    // Updates
    strings.insert("settings_section_updates".into(), "Mises à jour".into());
    strings.insert(
        "settings_updates_desc".into(),
        "Gérez la vérification et le canal des mises à jour.".into(),
    );
    strings.insert(
        "settings_auto_check_updates".into(),
        "Vérifier automatiquement".into(),
    );
    strings.insert(
        "settings_auto_check_updates_desc".into(),
        "Vérifie les nouvelles versions au lancement.".into(),
    );
    strings.insert(
        "settings_check_updates".into(),
        "Vérifier les mises à jour".into(),
    );
    // Privacy
    // Appearance
    strings.insert(
        "settings_appearance_title".into(),
        "Paramètres d'apparence".into(),
    );
    strings.insert(
        "settings_appearance_desc".into(),
        "Ajustez le thème, les accents et les effets visuels.".into(),
    );
    strings.insert("settings_section_theme".into(), "Thème".into());
    // Theme families
    // New theme families
    // Colors & accents
    strings.insert(
        "settings_section_colors".into(),
        "Couleurs & accents".into(),
    );
    strings.insert(
        "settings_auto_accent".into(),
        "Accent automatique selon le fond".into(),
    );
    strings.insert(
        "settings_auto_accent_desc".into(),
        "Adapte automatiquement l'accent aux arrière-plans.".into(),
    );
    strings.insert("settings_section_typography".into(), "Typographie".into());
    strings.insert(
        "settings_typography_desc".into(),
        "Configurez la police et la taille du texte.".into(),
    );
    strings.insert("settings_font_size".into(), "Taille du texte".into());
    strings.insert(
        "settings_font_size_desc".into(),
        "Taille de base du texte.".into(),
    );
    strings.insert("settings_font_size_default".into(), "Par défaut".into());
    strings.insert("settings_font_size_small".into(), "Petit".into());
    strings.insert("settings_font_size_large".into(), "Grand".into());
    strings.insert("settings_font_size_xlarge".into(), "Très grand".into());
    strings.insert(
        "settings_section_effects".into(),
        "Arrière-plans & effets".into(),
    );
    strings.insert(
        "settings_effects_desc".into(),
        "Gérez les animations et effets visuels.".into(),
    );
    strings.insert("settings_animations".into(), "Animations".into());
    strings.insert(
        "settings_animations_desc".into(),
        "Activer les transitions animées.".into(),
    );
    strings.insert("settings_section_preview".into(), "Aperçu".into());
    strings.insert(
        "settings_preview_card".into(),
        "Carte de prévisualisation".into(),
    );
    strings.insert(
        "settings_preview_summary".into(),
        "Thème: Sombre · Accent: Bleu · Texte: Par défaut · Effets: Activés".into(),
    );
    // Accessibility
    strings.insert(
        "settings_accessibility_title".into(),
        "Paramètres d'accessibilité".into(),
    );
    strings.insert(
        "settings_accessibility_desc".into(),
        "Facilitez la lecture, la navigation et la lecture média.".into(),
    );
    strings.insert("settings_section_vision".into(), "Vision".into());
    strings.insert(
        "settings_vision_desc".into(),
        "Options pour améliorer la lisibilité.".into(),
    );
    strings.insert("settings_high_contrast".into(), "Contraste élevé".into());
    strings.insert(
        "settings_high_contrast_desc".into(),
        "Augmente le contraste des éléments.".into(),
    );
    strings.insert("settings_text_size_a11y".into(), "Taille du texte".into());
    strings.insert(
        "settings_text_size_a11y_desc".into(),
        "Ajustez la taille du texte pour le confort.".into(),
    );
    strings.insert("settings_section_motion".into(), "Mouvement".into());
    strings.insert(
        "settings_motion_desc".into(),
        "Réduisez les animations pour le confort.".into(),
    );
    strings.insert(
        "settings_reduce_motion".into(),
        "Réduire les animations".into(),
    );
    strings.insert(
        "settings_reduce_motion_desc".into(),
        "Limite les transitions et mouvements.".into(),
    );
    strings.insert(
        "settings_section_navigation".into(),
        "Navigation & interaction".into(),
    );
    strings.insert(
        "settings_navigation_desc".into(),
        "Options de navigation au clavier et interaction.".into(),
    );
    strings.insert("settings_keyboard_nav".into(), "Navigation clavier".into());
    strings.insert(
        "settings_keyboard_nav_desc".into(),
        "Naviguer avec Tab et les flèches.".into(),
    );
    strings.insert("settings_section_reading".into(), "Lecture".into());
    strings.insert(
        "settings_reading_desc".into(),
        "Options de confort de lecture.".into(),
    );
    strings.insert("settings_dyslexia_font".into(), "Police dyslexie".into());
    strings.insert(
        "settings_dyslexia_font_desc".into(),
        "Utiliser une police adaptée à la dyslexie.".into(),
    );
    // Storage
    strings.insert("settings_storage_title".into(), "Stockage".into());
    strings.insert(
        "settings_storage_desc".into(),
        "Gérez l'emplacement des applications et du cache.".into(),
    );
    strings.insert("settings_section_scan".into(), "Scan".into());
    strings.insert(
        "settings_scan_desc".into(),
        "Configurez les dossiers analysés au démarrage.".into(),
    );
    strings.insert("settings_startup".into(), "Scanner au démarrage".into());
    strings.insert(
        "settings_startup_desc".into(),
        "Met à jour la bibliothèque au démarrage.".into(),
    );
    strings.insert("settings_section_install".into(), "Installation".into());
    strings.insert("settings_local_apps".into(), "Applications locales".into());
    strings.insert("settings_colony_repos".into(), "Dépôts Colony".into());
    strings.insert("settings_favorites".into(), "Favoris".into());
    // Placeholders
    // About
    strings.insert("settings_about_title".into(), "À propos de Colony".into());
    strings.insert("settings_about".into(), "À propos".into());
    // Launcher self-update
    strings.insert(
        "launcher_update_available".into(),
        "Colony {version} est disponible !".into(),
    );
    strings.insert(
        "launcher_update_available_short".into(),
        "\u{f0aa}  Mise à jour {version}".into(),
    );
    strings.insert(
        "launcher_update_ready".into(),
        "Mise à jour prête. Cliquez pour relancer Colony.".into(),
    );
    strings.insert(
        "launcher_restart_to_update".into(),
        "\u{f021}  Relancer pour mettre à jour".into(),
    );
    strings.insert(
        "launcher_download_update".into(),
        "Télécharger la mise à jour {version}".into(),
    );
    strings.insert(
        "check_launcher_updates".into(),
        "Vérifier les mises à jour".into(),
    );
    strings.insert("launcher_up_to_date".into(), "Colony est à jour".into());
    strings.insert("update_all".into(), "Tout mettre à jour ({count})".into());
    strings.insert("whats_new".into(), "Nouveautés de {version}".into());
    strings.insert("view_on_github".into(), "Voir sur GitHub".into());
    strings.insert("installed_version".into(), "Installé : {version}".into());
    strings.insert("launch_action".into(), "Lancer".into());
    strings.insert("section_security".into(), "Sécurité".into());
    strings.insert("language_changed".into(), "Langue changée".into());
    strings.insert("clear_caches".into(), "Vider les caches du store".into());
    strings.insert("clear_caches_desc".into(), "Supprime les descriptions et icônes mises en cache (elles se re-téléchargent au prochain rafraîchissement). Les applications installées ne sont pas touchées.".into());
    strings.insert(
        "caches_cleared".into(),
        "{count} cache(s) supprimé(s)".into(),
    );
    strings.insert("launcher_update_system_managed".into(), "Mise à jour {version} disponible - cette installation est gérée par le gestionnaire de paquets, mettez à jour via « pacman -Syu » (colony-bin)".into());
    // Detail tabs
    strings.insert("tab_readme".into(), "ReadMe".into());
    strings.insert("tab_license".into(), "License".into());
    strings.insert("tab_changelog".into(), "Changelog".into());
    strings.insert("tab_not_available".into(), "Non disponible".into());
}
