# @APP@ @VERSION@ — התקנה / Installation

@APP@ (חלופה ל-Photoshop) — עריכת תמונות ועיבוד גרפי. מתאים לריטוש, לקומפוזיציה ולכל מה
שביניהם, בלי לשלם על כל חודש שבו פתחתם את התוכנה.

[English below](#english)

## עברית

### Windows
בקובץ ה-ZIP של Windows יש שתי אפשרויות. בחרו אחת:

1. **התקנה רגילה (מומלץ):** לחצו פעמיים על הקובץ שמסתיים ב-`windows-x64.msi`
   (במחשב ישן של 32 סיביות: `windows-x86.msi`) ועקבו אחרי האשף.
   @APP@ יופיע בתפריט התחל.
2. **גרסה ניידת, בלי התקנה:** חלצו את הקובץ שמסתיים ב-`portable.zip` לתיקייה כלשהי
   (גם לדיסק און קי) והפעילו את `photocraft.exe`. ההגדרות נשמרות באותה תיקייה.

אם מופיעה ההודעה "Windows הגן על המחשב שלך": לחצו **מידע נוסף** ואז **הפעל בכל זאת**.
ההודעה מופיעה כי הקובץ לא נחתם בתעודת מפתח בתשלום.

### Mac (macOS 11 ומעלה, מעבדי Intel ו-Apple)
1. לחצו פעמיים על קובץ ה-`.dmg`.
2. גררו את **@APP@** לתיקייה **Applications**.
3. בפתיחה הראשונה: לחצו על האפליקציה **בלחצן הימני ← פתח ← פתח**.
   אם macOS עדיין חוסם: **הגדרות מערכת ← פרטיות ואבטחה ← "פתח בכל זאת"**.
   האזהרה מופיעה כי האפליקציה לא עברה אישור (notarization) של Apple.

### Linux (x86_64 או aarch64/ARM)
בחרו את הקובץ שמתאים למחשב שלכם (`x86_64` למחשב רגיל, `aarch64` ל-ARM):

- **AppImage (כל הפצה):** `chmod +x @APP_FILE@-*.AppImage` ואז הפעילו אותו בלחיצה כפולה.
- **Ubuntu / Debian / Mint:** `sudo apt install ./@APP_FILE@-*.deb`
- **Fedora / openSUSE:** `sudo dnf install ./@APP_FILE@-*.rpm`

### שפת הממשק
התוכנה עוברת לעברית אוטומטית כשהמחשב מוגדר לעברית. כדי לבחור ידנית:
**עריכה ← העדפות ← ממשק ← שפה ← עברית** (Edit › Preferences › Interface › Language).

### בדיקת הקבצים
`SHA256SUMS.txt` (בדף ההורדה) מכיל טביעות אצבע לכל קובץ, לבדיקה שההורדה שלמה.

---

## English

### Windows
The Windows ZIP has two options; pick one:

1. **Installer (recommended):** double-click the file ending in `windows-x64.msi`
   (on an old 32-bit PC: `windows-x86.msi`) and follow the wizard. @APP@ appears in the Start menu.
2. **Portable, no install:** extract the file ending in `portable.zip` anywhere (a USB stick works)
   and run `photocraft.exe`. Settings are kept in that folder.

If Windows shows "Windows protected your PC", click **More info**, then **Run anyway**. The warning
appears because the files are not signed with a paid code-signing certificate.

### Mac (macOS 11 or later, Intel and Apple silicon)
1. Double-click the `.dmg` file.
2. Drag **@APP@** into **Applications**.
3. The first time: **right-click the app → Open → Open**. If macOS still blocks it, go to
   **System Settings → Privacy & Security → Open Anyway**. The warning appears because the app is
   not notarized by Apple.

### Linux (x86_64 or aarch64/ARM)
Pick the file for your machine (`x86_64` for a regular PC, `aarch64` for ARM):

- **AppImage (any distribution):** `chmod +x @APP_FILE@-*.AppImage`, then double-click it.
- **Ubuntu / Debian / Mint:** `sudo apt install ./@APP_FILE@-*.deb`
- **Fedora / openSUSE:** `sudo dnf install ./@APP_FILE@-*.rpm`

### Interface language
The interface switches to Hebrew automatically on a Hebrew system. To choose it yourself:
**Edit › Preferences › Interface › Language → עברית**.

### Checking the files
`SHA256SUMS.txt` (on the download page) lists a checksum for every file.

---

@APP@ is based on PhotoCraft (https://github.com/storytold/photocraft), open source under
MIT OR Apache-2.0. Source code: https://github.com/prockstem/photocraftheb
