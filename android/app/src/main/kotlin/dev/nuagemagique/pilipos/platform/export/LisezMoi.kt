package dev.nuagemagique.pilipos.platform.export

/**
 * The text of `LISEZMOI.txt`, written in French in every export (the controller reads French). It names no
 * client and no brand: it says what each file is and how to check the export with `shasum -a 256 -c`.
 */
object LisezMoi {
    /** Name of the manifest file the text talks about. */
    const val MANIFEST_NAME = "MANIFEST.sha256"

    /**
     * The text for an export made at [exportedAt] (local time in Europe/Brussels, already formatted) by the
     * device [deviceId] with the POS id [posId] (null when the engine did not say).
     */
    fun render(
        exportedAt: String,
        deviceId: String,
        posId: String?,
    ): String =
        buildString {
            appendLine("EXPORT DES DONNÉES DE LA CAISSE ENREGISTREUSE")
            appendLine("=============================================")
            appendLine()
            appendLine("Date de l'export : $exportedAt (heure de Bruxelles)")
            appendLine("Identifiant de l'appareil (deviceId) : $deviceId")
            appendLine("Identifiant de la caisse (posId) : ${posId ?: "inconnu"}")
            appendLine()
            appendLine("Contenu de ce dossier")
            appendLine("---------------------")
            appendLine("pos.db")
            appendLine("    Copie complète et cohérente de la base de données SQLite de la caisse (journal fiscal compris).")
            appendLine("    Elle a été faite au moment de l'export avec l'API de sauvegarde en ligne de SQLite : elle")
            appendLine("    s'ouvre telle quelle avec sqlite3 et ne dépend d'aucun autre fichier.")
            appendLine("backups/")
            appendLine("    Les autres sauvegardes de la base gardées sur la caisse, faites après chaque rapport Z ou lors")
            appendLine("    d'un export. La caisse garde les 7 plus récentes, dont la copie pos.db de cet export.")
            appendLine("backups.txt")
            appendLine("    Inventaire de toutes les sauvegardes présentes sur la caisse avant l'export, y compris celles")
            appendLine("    qui ne sont pas copiées dans backups/. Une ligne par sauvegarde, trois colonnes séparées par")
            appendLine("    une tabulation : nom du fichier, taille en octets, empreinte SHA-256.")
            appendLine("logs/")
            appendLine("    Les journaux techniques du moteur de la caisse.")
            appendLine("audit/")
            appendLine("    Le journal d'audit de la plateforme (une ligne JSON par événement : exports, mises à jour,")
            appendLine("    redémarrages). Ce n'est pas le journal fiscal.")
            appendLine("identity.json")
            appendLine("    L'identité de l'installation : version, empreintes SHA-1 et SHA-256 de l'application installée,")
            appendLine("    empreinte SHA-1 de la bibliothèque du moteur, certificat de signature, appareil, stockage.")
            appendLine("LISEZMOI.txt")
            appendLine("    Ce fichier.")
            appendLine(MANIFEST_NAME)
            appendLine("    La liste des autres fichiers avec leur empreinte SHA-256, au format de la commande sha256sum.")
            appendLine("    Il est écrit en dernier : s'il manque, l'export est incomplet.")
            appendLine()
            appendLine("Vérifier l'export")
            appendLine("-----------------")
            appendLine("Depuis ce dossier, sur un poste Linux ou macOS :")
            appendLine()
            appendLine("    shasum -a 256 -c $MANIFEST_NAME")
            appendLine()
            appendLine("(sous Linux, sha256sum -c $MANIFEST_NAME donne le même résultat). Chaque ligne doit afficher OK.")
            appendLine("Pour contrôler la base : sqlite3 pos.db \"PRAGMA integrity_check;\" doit répondre ok.")
        }
}
