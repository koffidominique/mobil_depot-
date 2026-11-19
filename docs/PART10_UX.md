# PARTIE 10 — Design et expérience utilisateur

## 1. Direction artistique

Le thème `src/theme.ts` conserve une identité DEPOTPRO PREMIUM : vert forêt, or,Surface claire, typographie hiérarchisée et cartes arrondies. Deux thèmes cohabitent via `themeMode` (Zustand) : clair et sombre, avec des couleurs sémantiques (`background`, `surface`, `text`, `subtitle`, `border`, `primary`, `danger`, `gold`).

Les composants partagés `src/components/ui.tsx` fournissent écran, carte, métrique, bouton, champ, ligne, états de chargement/vide et toast de retour. Aucun tableau de bureau n'est reproduit : les indicateurs sont des cartes lisibles sur mobile.

## 2. Responsive

`src/core/design.ts` calcule la mise en page de façon pure et testable :

| Largeur | Comportement |
| --- | --- |
| < 360 px | 1 colonne, padding 12 px, cibles tactiles ≥ 44 px |
| 360–599 px | 2 colonnes, padding 18 px |
| ≥ 600 px | 3 colonnes |
| Paysage < 700 px | 1 colonne pour préserver la lisibilité |

`useWindowDimensions` réagit aux rotations. `Screen` utilise `keyboardShouldPersistTaps="handled"` et `keyboardDismissMode="on-drag"` ; les champs ont `returnKeyType="done"` et `blurOnSubmit`. SafeAreaProvider englobe l'application.

## 3. Animations discrètes

`AnimatedPressable` applique une légère rétractation (0,98 sur 90 ms) aux boutons d'action. `useReducedMotion` interroge `AccessibilityInfo.isReduceMotionEnabled` et écoute `reduceMotionChanged` : dès que l'utilisateur préfère moins de mouvement, l'animation est désactivée. Aucune animation n'est ajoutée au chemin critique de caisse.

## 4. Accessibilité

- Contrastes AA vérifiés par calculateur WCAG dans `__tests__/part10-design.test.ts` pour les deux thèmes (texte/surface, sous-titre/fond, texte bouton/fond, danger).
- Cibles tactiles minimales de 44 px ; libellés `accessibilityLabel` sur champs, boutons, métriques, lignes, banned d'erreur et entrées de menu.
- `accessibilityRole` : `header`, `button`, `tab`, `alert` ; `accessibilityState` pour désactivé/sélectionné ; `accessibilityLiveRegion="polite"` pour messages et toast.
- Les messages d'erreur ne reposent pas uniquement sur la couleur : préfixe `!`, `i` ou `✓`.
- `FeedbackToast` affiche globalement le retour après chaque action ; `emptyState` et `LoadingState` couvrent les écrans sans données et le chargement.

## 5. Vérifications réelles

`tsc`, typecheck web, ESLint, Jest **12 suites / 39 tests** (dont responsive et contrastes), codegen, bundle et aperçu web JSDOM (connexion puis accueil rendu sans erreur) : OK.

## 6. Limites

La matrice physique (petits Android, Android standard, grands écrans, iPhone, orientation, claviers) n'a pas pu être exécutée : aucun émulateur, SDK Android ou Xcode dans cet environnement. Les règles responsive sont donc vérifiées par calcul et par l'aperçu navigateur à 460 px, pas sur appareil. Une passe de test manuelle reste nécessaire avant release sur la matrice complète.
