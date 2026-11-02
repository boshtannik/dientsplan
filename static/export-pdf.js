/* Duty Scheduler — export-pdf.js
   Landscape A4 Duty Schedule Table Export (DE / RU)
*/

const PDF_TRANSLATIONS = {
  de: {
    docTitle: 'Dienstplan',
    periodPrefix: 'Zeitraum:',
    buildingDefault: 'Loggia 4',
    colName: 'Diensthabende(r) / Name',
    colRoom: 'Bereich / Raum',
    weekdays: ['So', 'Mo', 'Di', 'Mi', 'Do', 'Fr', 'Sa'],
    footerNote: 'Hinweis: Bitte die Dienste gewissenhaft durchführen und abhaken (✓).',
    generatedAt: 'Erstellt am:',
    noData: 'Keine Daten',
    printBtn: '🖨️ Drucken / Als PDF speichern',
    closeBtn: '✕ Schließen',
    rooms: {
      restaurant: 'Restaurant / Speisesaal',
      kitchen: 'Küche',
      toilet: 'Toiletten (WC)',
      shower: 'Duschen',
      washbasin: 'Waschbecken',
      trash: 'Müll & Außenbereich',
      street: 'Außenbereich',
      ktln: 'Heizungsraum'
    },
    roomRuToDe: {
      'Ресторан': 'Restaurant / Speisesaal',
      'Кухня': 'Küche',
      'Туалет': 'Toiletten (WC)',
      'Душ': 'Duschen',
      'Умывальник': 'Waschbecken',
      'Мусор + Улица': 'Müll & Außenbereich',
      'Улица': 'Außenbereich',
      'Котельная': 'Heizungsraum'
    }
  },
  ru: {
    docTitle: 'График дежурств',
    periodPrefix: 'Период:',
    buildingDefault: 'Лоджия 4',
    colName: 'Дежурный (ФИО)',
    colRoom: 'Помещение',
    weekdays: ['Вс', 'Пн', 'Вт', 'Ср', 'Чт', 'Пт', 'Сб'],
    footerNote: 'Отметка о выполнении дежурства: галочка (✓), крестик или подпись.',
    generatedAt: 'Сформировано:',
    noData: 'Нет данных',
    printBtn: '🖨️ Печать / Сохранить в PDF',
    closeBtn: '✕ Закрыть',
    rooms: {
      restaurant: 'Ресторан',
      kitchen: 'Кухня',
      toilet: 'Туалет',
      shower: 'Душ',
      washbasin: 'Умывальник',
      trash: 'Мусор + Улица',
      street: 'Улица',
      ktln: 'Котельная'
    },
    roomRuToDe: {}
  }
};

function formatDayHeader(startDateStr, dayOffset, lang) {
  const t = PDF_TRANSLATIONS[lang] || PDF_TRANSLATIONS.ru;
  if (!startDateStr) return `Д${dayOffset + 1}`;
  const parts = startDateStr.split('-').map(Number);
  if (parts.length < 3 || isNaN(parts[0])) return `Д${dayOffset + 1}`;
  const d = new Date(parts[0], parts[1] - 1, parts[2] + dayOffset);
  const wd = t.weekdays[d.getDay()];
  const dd = String(d.getDate()).padStart(2, '0');
  const mm = String(d.getMonth() + 1).padStart(2, '0');
  return `${wd} ${dd}.${mm}`;
}

function getLocalizedRoomName(room, lang) {
  const t = PDF_TRANSLATIONS[lang];
  if (lang === 'de') {
    if (room.key && t.rooms[room.key]) return t.rooms[room.key];
    if (t.roomRuToDe[room.name]) return t.roomRuToDe[room.name];
  } else {
    if (room.key && t.rooms[room.key]) return t.rooms[room.key];
  }
  return room.name;
}

function buildDutyRows(viewData, lang) {
  const rows = [];
  const rooms = viewData.rooms || [];
  const days = viewData.days || [];

  for (const room of rooms) {
    const assignedMap = new Map();

    for (const day of days) {
      for (const a of (day.assignments || [])) {
        if (a.room_id === room.id) {
          if (!assignedMap.has(a.person_id)) {
            assignedMap.set(a.person_id, {
              name: a.person_name,
              days: new Set()
            });
          }
          assignedMap.get(a.person_id).days.add(day.day);
        }
      }
    }

    const roomTitle = getLocalizedRoomName(room, lang);

    // Add row for each assigned person
    for (const [pid, info] of assignedMap) {
      rows.push({
        personName: info.name,
        roomName: roomTitle,
        isBlank: false
      });
    }

    // If fewer assigned people than required, add empty rows for write-in
    const emptyCount = Math.max(0, (room.required || 1) - assignedMap.size);
    for (let i = 0; i < emptyCount; i++) {
      rows.push({
        personName: '_______________________',
        roomName: roomTitle,
        isBlank: true
      });
    }
  }

  return rows;
}

function generatePrintHtml(viewData, initialLang) {
  const viewJson = JSON.stringify(viewData).replace(/</g, '\\u003c');
  const translationsJson = JSON.stringify(PDF_TRANSLATIONS).replace(/</g, '\\u003c');

  return `<!DOCTYPE html>
<html lang="${initialLang}">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title id="docTitle">Dienstplan</title>
<style>
  @page {
    size: A4 landscape;
    margin: 8mm 10mm 8mm 10mm;
  }
  * {
    box-sizing: border-box;
    margin: 0;
    padding: 0;
  }
  body {
    font-family: 'Segoe UI', -apple-system, BlinkMacSystemFont, Roboto, 'Helvetica Neue', Arial, sans-serif;
    background: #f8fafc;
    color: #0f172a;
    -webkit-print-color-adjust: exact;
    print-color-adjust: exact;
  }
  
  /* On-screen control bar */
  .no-print-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    background: #0f172a;
    color: #f8fafc;
    padding: 10px 20px;
    box-shadow: 0 2px 10px rgba(0,0,0,0.3);
    position: sticky;
    top: 0;
    z-index: 1000;
    font-size: 14px;
  }
  .no-print-bar .btn-group {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .bar-btn {
    padding: 6px 14px;
    font-size: 13px;
    font-weight: 600;
    border-radius: 6px;
    border: 1px solid #334155;
    background: #1e293b;
    color: #f8fafc;
    cursor: pointer;
    transition: all .15s;
  }
  .bar-btn:hover {
    background: #334155;
  }
  .bar-btn.primary {
    background: #38bdf8;
    color: #000;
    border-color: #38bdf8;
  }
  .bar-btn.primary:hover {
    background: #0ea5e9;
  }
  .bar-btn.active {
    background: #38bdf8;
    color: #000;
    border-color: #38bdf8;
  }

  /* Sheet container */
  .sheet {
    max-width: 1120px;
    margin: 20px auto;
    background: #fff;
    padding: 24px 30px;
    box-shadow: 0 4px 20px rgba(0,0,0,0.08);
    border-radius: 8px;
  }

  .header-box {
    display: flex;
    justify-content: space-between;
    align-items: flex-end;
    margin-bottom: 16px;
    padding-bottom: 10px;
    border-bottom: 2.5px solid #000;
  }
  .title-main {
    font-size: 22pt;
    font-weight: 800;
    letter-spacing: -0.02em;
    color: #000;
    line-height: 1.1;
  }
  .title-sub {
    font-size: 11pt;
    color: #475569;
    margin-top: 4px;
    font-weight: 500;
  }
  .header-meta {
    text-align: right;
  }
  .meta-period {
    font-size: 14pt;
    font-weight: 700;
    color: #000;
  }
  .meta-building {
    font-size: 10.5pt;
    color: #475569;
    margin-top: 3px;
  }

  /* Duty Table */
  .duty-table {
    width: 100%;
    border-collapse: collapse;
    table-layout: fixed;
    margin-bottom: 12px;
  }
  .duty-table th, .duty-table td {
    border: 1.5px solid #000;
    text-align: center;
    vertical-align: middle;
  }
  .duty-table thead {
    display: table-header-group;
  }
  .duty-table tr {
    page-break-inside: avoid;
  }
  .duty-table th {
    background: #f1f5f9;
    color: #000;
    font-weight: 700;
    font-size: 10.5pt;
    padding: 10px 4px;
  }
  .duty-table th.col-name {
    width: 26%;
    text-align: left;
    padding-left: 12px;
  }
  .duty-table th.col-room {
    width: 24%;
    text-align: left;
    padding-left: 12px;
  }
  .duty-table th.col-day {
    width: calc(50% / 7);
  }

  .duty-table td.cell-name {
    text-align: left;
    padding: 10px 12px;
    font-weight: 600;
    font-size: 11pt;
    color: #000;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .duty-table td.cell-room {
    text-align: left;
    padding: 10px 12px;
    font-size: 10.5pt;
    color: #1e293b;
    font-weight: 500;
  }
  .duty-table td.cell-day {
    height: 64px;
    background: #fff;
    cursor: default;
  }

  .footer-box {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-top: 14px;
    padding-top: 8px;
    border-top: 1px dashed #cbd5e1;
    font-size: 8.5pt;
    color: #64748b;
  }

  @media print {
    body {
      background: #fff;
      color: #000;
    }
    .no-print-bar {
      display: none !important;
    }
    .sheet {
      max-width: 100%;
      margin: 0;
      padding: 0;
      box-shadow: none;
      border-radius: 0;
    }
    .duty-table th {
      background: #f1f5f9 !important;
    }
  }
</style>
</head>
<body>

<div class="no-print-bar">
  <div class="btn-group">
    <button class="bar-btn primary" id="btnPrint" onclick="window.print()">🖨️ Drucken / Als PDF speichern</button>
    <span style="color:#64748b;margin:0 4px">|</span>
    <button class="bar-btn" id="btnLangDe" onclick="setLanguage('de')">🇩🇪 Deutsch</button>
    <button class="bar-btn" id="btnLangRu" onclick="setLanguage('ru')">🇷🇺 Русский</button>
  </div>
  <div class="btn-group">
    <button class="bar-btn" id="btnClose" onclick="window.close()">✕ Schließen</button>
  </div>
</div>

<div class="sheet">
  <div class="header-box">
    <div>
      <div class="title-main" id="titleMain">Dienstplan</div>
      <div class="title-sub" id="titleSub">Loggia 4</div>
    </div>
    <div class="header-meta">
      <div class="meta-period" id="metaPeriod"></div>
      <div class="meta-building" id="metaBuilding"></div>
    </div>
  </div>

  <table class="duty-table">
    <thead id="tableHead"></thead>
    <tbody id="tableBody"></tbody>
  </table>

  <div class="footer-box">
    <div id="footerNote"></div>
    <div id="footerGen"></div>
  </div>
</div>

<script>
const VIEW = ${viewJson};
const TRANSLATIONS = ${translationsJson};
let currentLang = '${initialLang}';

function formatDayHeaderLocal(startDateStr, dayOffset, lang) {
  const t = TRANSLATIONS[lang] || TRANSLATIONS.ru;
  if (!startDateStr) return 'Д' + (dayOffset + 1);
  const parts = startDateStr.split('-').map(Number);
  if (parts.length < 3 || isNaN(parts[0])) return 'Д' + (dayOffset + 1);
  const d = new Date(parts[0], parts[1] - 1, parts[2] + dayOffset);
  const wd = t.weekdays[d.getDay()];
  const dd = String(d.getDate()).padStart(2, '0');
  const mm = String(d.getMonth() + 1).padStart(2, '0');
  return wd + ' ' + dd + '.' + mm;
}

function formatDateRange(startDateStr, numDays) {
  if (!startDateStr) return '';
  const parts = startDateStr.split('-').map(Number);
  if (parts.length < 3 || isNaN(parts[0])) return startDateStr;
  const start = new Date(parts[0], parts[1] - 1, parts[2]);
  const end = new Date(parts[0], parts[1] - 1, parts[2] + Math.max(0, (numDays || 1) - 1));
  const f = d => String(d.getDate()).padStart(2, '0') + '.' + String(d.getMonth() + 1).padStart(2, '0') + '.' + d.getFullYear();
  return f(start) + ' — ' + f(end);
}

function getLocalizedRoomNameLocal(room, lang) {
  const t = TRANSLATIONS[lang];
  if (lang === 'de') {
    if (room.key && t.rooms[room.key]) return t.rooms[room.key];
    if (t.roomRuToDe[room.name]) return t.roomRuToDe[room.name];
  } else {
    if (room.key && t.rooms[room.key]) return t.rooms[room.key];
  }
  return room.name;
}

function buildDutyRowsLocal(viewData, lang) {
  const rows = [];
  const rooms = viewData.rooms || [];
  const days = viewData.days || [];

  for (const room of rooms) {
    const assignedMap = new Map();

    for (const day of days) {
      for (const a of (day.assignments || [])) {
        if (a.room_id === room.id) {
          if (!assignedMap.has(a.person_id)) {
            assignedMap.set(a.person_id, {
              name: a.person_name,
              days: new Set()
            });
          }
          assignedMap.get(a.person_id).days.add(day.day);
        }
      }
    }

    const roomTitle = getLocalizedRoomNameLocal(room, lang);

    for (const [pid, info] of assignedMap) {
      rows.push({
        personName: info.name,
        roomName: roomTitle,
        isBlank: false
      });
    }

    const emptyCount = Math.max(0, (room.required || 1) - assignedMap.size);
    for (let i = 0; i < emptyCount; i++) {
      rows.push({
        personName: '_______________________',
        roomName: roomTitle,
        isBlank: true
      });
    }
  }

  return rows;
}

function render(lang) {
  currentLang = lang;
  const t = TRANSLATIONS[lang];
  const p = VIEW.period || {};
  const b = VIEW.building || {};
  const days = VIEW.days || [];

  // Doc title for PDF default filename
  const pClean = (p.name || '').replace(/\\s+/g, '');
  const bClean = (b.name || t.buildingDefault).replace(/\\s+/g, '');
  document.title = (lang === 'de' ? 'Dienstplan_' : 'График_дежурств_') + bClean + (pClean ? '_' + pClean : '');

  // UI buttons
  document.getElementById('btnPrint').textContent = t.printBtn;
  document.getElementById('btnClose').textContent = t.closeBtn;
  document.getElementById('btnLangDe').className = 'bar-btn' + (lang === 'de' ? ' active' : '');
  document.getElementById('btnLangRu').className = 'bar-btn' + (lang === 'ru' ? ' active' : '');

  // Header
  document.getElementById('titleMain').textContent = t.docTitle;
  document.getElementById('titleSub').textContent = b.name || t.buildingDefault;
  document.getElementById('metaPeriod').textContent = (p.name || '');
  document.getElementById('metaBuilding').textContent = formatDateRange(p.start_date, p.num_days || days.length);

  // Table Head
  let thHtml = '<tr><th class="col-name">' + t.colName + '</th><th class="col-room">' + t.colRoom + '</th>';
  for (let d = 0; d < days.length; d++) {
    const dayLabel = formatDayHeaderLocal(p.start_date, d, lang);
    thHtml += '<th class="col-day">' + dayLabel + '</th>';
  }
  thHtml += '</tr>';
  document.getElementById('tableHead').innerHTML = thHtml;

  // Table Body
  const rows = buildDutyRowsLocal(VIEW, lang);
  let tbHtml = '';
  if (rows.length === 0) {
    tbHtml = '<tr><td colspan="' + (2 + days.length) + '" style="padding:24px;color:#94a3b8;font-style:italic">' + t.noData + '</td></tr>';
  } else {
    for (const r of rows) {
      tbHtml += '<tr>';
      tbHtml += '<td class="cell-name">' + (r.isBlank ? '<span style="color:#94a3b8">' + r.personName + '</span>' : escapeHtml(r.personName)) + '</td>';
      tbHtml += '<td class="cell-room">' + escapeHtml(r.roomName) + '</td>';
      for (let d = 0; d < days.length; d++) {
        tbHtml += '<td class="cell-day"></td>';
      }
      tbHtml += '</tr>';
    }
  }
  document.getElementById('tableBody').innerHTML = tbHtml;

  // Footer
  document.getElementById('footerNote').textContent = t.footerNote;
  const now = new Date();
  const nowStr = String(now.getDate()).padStart(2,'0') + '.' + String(now.getMonth()+1).padStart(2,'0') + '.' + now.getFullYear();
  document.getElementById('footerGen').textContent = t.generatedAt + ' ' + nowStr;
}

function escapeHtml(s) {
  const d = document.createElement('div');
  d.textContent = s || '';
  return d.innerHTML;
}

function setLanguage(lang) {
  render(lang);
}

// Initial render
render(currentLang);

// Trigger print dialog after brief render delay
setTimeout(function() {
  window.print();
}, 350);
</script>
</body>
</html>`;
}

// Currently selected period data (set when the modal is opened)
let pdfExportView = null;

// Open Export Modal
function openPdfModal(viewData) {
  if (viewData) pdfExportView = viewData;
  let modal = document.getElementById('pdfModal');
  if (!modal) {
    modal = document.createElement('div');
    modal.className = 'mo';
    modal.id = 'pdfModal';
    modal.innerHTML = `
      <div class="md">
        <button class="cb" onclick="document.getElementById('pdfModal').classList.remove('on')">&times;</button>
        <h3 style="margin-bottom:10px">Экспорт расписания в PDF</h3>
        <div style="font-size:.82rem;color:var(--muted);margin-bottom:14px;line-height:1.4">
          Таблица для дежурств с пустыми клеточками для отметок (формат А4, ландшафт).
        </div>
        <label style="display:block;font-size:.78rem;color:var(--muted);margin-bottom:6px">Язык таблицы:</label>
        <div style="display:flex;flex-direction:column;gap:8px;margin-bottom:18px">
          <label style="display:flex;align-items:center;gap:8px;cursor:pointer;font-size:.88rem;color:var(--text);padding:8px 10px;background:var(--bg);border-radius:6px;border:1px solid var(--border)">
            <input type="radio" name="pdfExportLang" value="de" checked style="width:auto;margin:0">
            <span>🇩🇪 <b>Deutsch</b> (Немецкий)</span>
          </label>
          <label style="display:flex;align-items:center;gap:8px;cursor:pointer;font-size:.88rem;color:var(--text);padding:8px 10px;background:var(--bg);border-radius:6px;border:1px solid var(--border)">
            <input type="radio" name="pdfExportLang" value="ru" style="width:auto;margin:0">
            <span>🇷🇺 <b>Русский</b></span>
          </label>
        </div>
        <button class="btn btn-p" onclick="doExportPdfFromModal()" style="width:100%;padding:10px;font-size:.9rem;font-weight:700">
          📄 Открыть / Сохранить в PDF
        </button>
      </div>
    `;
    document.body.appendChild(modal);
  }
  modal.classList.add('on');
}

function doExportPdfFromModal() {
  const modal = document.getElementById('pdfModal');
  if (modal) modal.classList.remove('on');

  const selectedRadio = document.querySelector('input[name="pdfExportLang"]:checked');
  const lang = selectedRadio ? selectedRadio.value : 'de';

  const viewData = pdfExportView || window.view;
  if (!viewData) {
    alert('Нет загруженных данных расписания для экспорта');
    return;
  }

  const printHtml = generatePrintHtml(viewData, lang);
  const win = window.open('', '_blank');
  if (!win) {
    alert('Всплывающее окно заблокировано браузером. Разрешите всплывающие окна для экспорта.');
    return;
  }
  win.document.open();
  win.document.write(printHtml);
  win.document.close();
}

window.openPdfModal = openPdfModal;
window.doExportPdfFromModal = doExportPdfFromModal;
