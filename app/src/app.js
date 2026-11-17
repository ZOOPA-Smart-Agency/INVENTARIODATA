// The window: server and account -> (trust the server's certificate) -> review what will be sent -> done.
// Everything that touches the computer or the network runs in Rust (src-tauri/src/main.rs).
'use strict';

const I18N = {
  es: {
    title: 'Registro de equipos', subtitle: 'Añade este equipo al inventario o actualiza sus datos.',
    server: 'Dirección del inventario', server_ph: 'inventario.empresa.es', email: 'Correo o nombre de usuario', password: 'Contraseña',
    continue: 'Continuar', account_hint: 'Usa una cuenta de técnico o administrador del inventario. La contraseña no se guarda.',
    trust_title: 'Certificado propio del servidor',
    trust_text: 'Este servidor usa su propio certificado, que este equipo no reconoce. Confía en él solo si es el inventario de tu empresa.',
    fp_ca: 'Huella SHA-256 de su autoridad de certificación', fp_cert: 'Huella SHA-256 del certificado',
    trust_hint: 'Quien instaló el inventario puede confirmarte la huella. Solo se confiará en él para esta dirección y se recordará.',
    cancel: 'Cancelar', trust: 'Confiar y continuar',
    signed_as: 'Sesión iniciada como {name}', review_title: 'Datos de este equipo', reading: 'Leyendo los datos del equipo…',
    serial_manual: 'Número de serie (este equipo no lo indica)',
    new_device: 'Este equipo no está en el inventario. Se añadirá con estos datos:',
    name_is_tag: 'El nombre de este equipo, {name}, será su etiqueta de inventario.',
    name_invalid: 'En este inventario el nombre del equipo debe ser su etiqueta, pero «{name}» no es una etiqueta válida. Cámbiale el nombre a {tag}.',
    name_used: 'En este inventario el nombre del equipo debe ser su etiqueta, pero otro dispositivo ya usa {name}{used}. Cámbiale el nombre a {tag}.',
    name_differs: 'En este inventario el nombre del equipo debe ser su etiqueta: este equipo se llama {name}, pero su etiqueta es {tag}. Cámbiale el nombre a {tag}.',
    rename_btn: 'Cambiar el nombre a {tag}', name_settings: 'Abrir ajustes del sistema',
    renamed: 'Nombre cambiado a {tag}.', renamed_restart: 'Nombre cambiado a {tag}. Reinicia el equipo para que se aplique.',
    rename_cancelled: 'No se ha cambiado el nombre: se canceló la petición de permisos.', rename_failed: 'No se pudo cambiar el nombre: {detail}',
    no_settings: 'No se encontraron los ajustes del sistema. Cambia el nombre del equipo a mano.', bad_name: '{tag} no puede ser el nombre de un equipo. Cámbialo a mano.',
    existing: 'Este equipo ya está en el inventario como {tag}. Se actualizarán sus datos de hardware y software; el estado, la ubicación, la asignación y el resto no cambian.',
    tag: 'Etiqueta', type: 'Tipo', location: 'Ubicación',
    add: 'Añadir al inventario', update: 'Actualizar en el inventario', sign_out: 'Cambiar de cuenta',
    done_added: 'Añadido como {tag}', done_updated: '{tag} actualizado', done_text: 'Ya puedes cerrar esta ventana.',
    open: 'Ver en el inventario', close: 'Cerrar', version: 'Versión {v}',
    f_serial: 'Número de serie', f_name: 'Nombre del equipo', f_model: 'Modelo', f_cpu: 'Procesador', f_ram: 'Memoria',
    f_disks: 'Almacenamiento', f_gpus: 'Gráficos', f_screen: 'Pantalla', f_battery: 'Salud de la batería', f_os: 'Sistema',
    f_enc: 'Cifrado del disco', f_domain: 'Unido a dominio', f_nics: 'Red', yes: 'Sí', no: 'No', cores: '{n} núcleos',
    w_serial_missing: 'Este equipo no indica su número de serie. Escríbelo abajo (suele estar en una etiqueta del equipo).',
    w_serial_needs_admin: 'Para leer el número de serie y los módulos de memoria, abre la aplicación como administrador (sudo).',
    w_battery_needs_admin: 'La salud de la batería solo se puede leer abriendo la aplicación como administrador.',
    w_windows_tools_failed: 'Windows no ha respondido a las consultas de hardware (PowerShell). Faltan datos.',
    w_other: 'Algunos datos no se han podido leer.',
    e_login_err: 'Correo, nombre de usuario o contraseña incorrectos.', e_login_off: 'Esta cuenta está desactivada.',
    e_login_locked: 'Demasiados intentos fallidos. Espera unos minutos y vuelve a probar.',
    e_err_must_change: 'Esta cuenta tiene que cambiar la contraseña en la web del inventario antes de usarla aquí.',
    e_err_forbidden: 'Esta cuenta solo puede consultar el inventario. Hace falta una cuenta de técnico o administrador.',
    e_err_auth: 'La sesión ha caducado. Vuelve a iniciar sesión.', e_err_dup: 'Esa etiqueta, o su número identificador, ya la usa otro dispositivo.',
    e_err_required: 'Al inventario le falta algo que necesita (un número de serie, un tipo de dispositivo o una ubicación).',
    e_maintenance: 'El inventario se está restaurando o actualizando. Prueba de nuevo en unos minutos.',
    e_not_inventory: 'Esa dirección no parece un inventario.', e_err_invalid: 'El servidor no ha aceptado los datos.',
    e_network: 'No se ha podido conectar con el servidor. Comprueba la dirección y la red.',
    e_address: 'La dirección no es válida. Debe empezar por https://', e_fill: 'Rellena la dirección, el email y la contraseña.',
    e_serial: 'Escribe el número de serie del equipo.', e_other: 'Algo ha fallado. Vuelve a probar.',
  },
  ca: {
    title: "Registre d'equips", subtitle: "Afegeix aquest equip a l'inventari o actualitza'n les dades.",
    server: "Adreça de l'inventari", server_ph: 'inventari.empresa.cat', email: 'Correu o nom d’usuari', password: 'Contrasenya',
    continue: 'Continua', account_hint: "Fes servir un compte de tècnic o administrador de l'inventari. La contrasenya no es desa.",
    trust_title: 'Certificat propi del servidor',
    trust_text: "Aquest servidor fa servir el seu propi certificat, que aquest equip no reconeix. Confia-hi només si és l'inventari de la teva empresa.",
    fp_ca: "Empremta SHA-256 de la seva autoritat de certificació", fp_cert: 'Empremta SHA-256 del certificat',
    trust_hint: "Qui va instal·lar l'inventari et pot confirmar l'empremta. Només s'hi confiarà per a aquesta adreça i es recordarà.",
    cancel: 'Cancel·la', trust: 'Confia-hi i continua',
    signed_as: 'Sessió iniciada com a {name}', review_title: "Dades d'aquest equip", reading: "S'estan llegint les dades de l'equip…",
    serial_manual: "Número de sèrie (aquest equip no l'indica)",
    new_device: "Aquest equip no és a l'inventari. S'hi afegirà amb aquestes dades:",
    name_is_tag: 'El nom d’aquest equip, {name}, serà la seva etiqueta d’inventari.',
    name_invalid: 'En aquest inventari el nom de l’equip ha de ser la seva etiqueta, però «{name}» no és una etiqueta vàlida. Canvia-li el nom a {tag}.',
    name_used: 'En aquest inventari el nom de l’equip ha de ser la seva etiqueta, però un altre dispositiu ja fa servir {name}{used}. Canvia-li el nom a {tag}.',
    name_differs: 'En aquest inventari el nom de l’equip ha de ser la seva etiqueta: aquest equip es diu {name}, però la seva etiqueta és {tag}. Canvia-li el nom a {tag}.',
    rename_btn: 'Canvia el nom a {tag}', name_settings: 'Obre la configuració del sistema',
    renamed: 'Nom canviat a {tag}.', renamed_restart: 'Nom canviat a {tag}. Reinicia l’equip perquè s’apliqui.',
    rename_cancelled: 'No s’ha canviat el nom: s’ha cancel·lat la petició de permisos.', rename_failed: 'No s’ha pogut canviar el nom: {detail}',
    no_settings: 'No s’ha trobat la configuració del sistema. Canvia el nom de l’equip a mà.', bad_name: '{tag} no pot ser el nom d’un equip. Canvia’l a mà.',
    existing: "Aquest equip ja és a l'inventari com a {tag}. S'actualitzaran les dades de maquinari i programari; l'estat, la ubicació, l'assignació i la resta no canvien.",
    tag: 'Etiqueta', type: 'Tipus', location: 'Ubicació',
    add: "Afegeix a l'inventari", update: "Actualitza a l'inventari", sign_out: 'Canvia de compte',
    done_added: 'Afegit com a {tag}', done_updated: '{tag} actualitzat', done_text: 'Ja pots tancar aquesta finestra.',
    open: "Mostra a l'inventari", close: 'Tanca', version: 'Versió {v}',
    f_serial: 'Número de sèrie', f_name: "Nom de l'equip", f_model: 'Model', f_cpu: 'Processador', f_ram: 'Memòria',
    f_disks: 'Emmagatzematge', f_gpus: 'Gràfics', f_screen: 'Pantalla', f_battery: 'Salut de la bateria', f_os: 'Sistema',
    f_enc: 'Xifratge del disc', f_domain: 'Unit a domini', f_nics: 'Xarxa', yes: 'Sí', no: 'No', cores: '{n} nuclis',
    w_serial_missing: "Aquest equip no indica el número de sèrie. Escriu-lo a sota (sol estar en una etiqueta de l'equip).",
    w_serial_needs_admin: "Per llegir el número de sèrie i els mòduls de memòria, obre l'aplicació com a administrador (sudo).",
    w_battery_needs_admin: "La salut de la bateria només es pot llegir obrint l'aplicació com a administrador.",
    w_windows_tools_failed: 'Windows no ha respost a les consultes de maquinari (PowerShell). Falten dades.',
    w_other: "No s'han pogut llegir algunes dades.",
    e_login_err: 'Correu, nom d’usuari o contrasenya incorrectes.', e_login_off: 'Aquest compte està desactivat.',
    e_login_locked: 'Massa intents fallits. Espera uns minuts i torna-ho a provar.',
    e_err_must_change: "Aquest compte ha de canviar la contrasenya al web de l'inventari abans de fer-lo servir aquí.",
    e_err_forbidden: "Aquest compte només pot consultar l'inventari. Cal un compte de tècnic o administrador.",
    e_err_auth: 'La sessió ha caducat. Torna a iniciar sessió.', e_err_dup: 'Aquesta etiqueta, o el seu número identificador, ja la fa servir un altre dispositiu.',
    e_err_required: "A l'inventari li falta una cosa que necessita (un número de sèrie, un tipus de dispositiu o una ubicació).",
    e_maintenance: "L'inventari s'està restaurant o actualitzant. Torna-ho a provar d'aquí a uns minuts.",
    e_not_inventory: 'Aquesta adreça no sembla un inventari.', e_err_invalid: 'El servidor no ha acceptat les dades.',
    e_network: "No s'ha pogut connectar amb el servidor. Comprova l'adreça i la xarxa.",
    e_address: "L'adreça no és vàlida. Ha de començar per https://", e_fill: "Omple l'adreça, el correu i la contrasenya.",
    e_serial: "Escriu el número de sèrie de l'equip.", e_other: 'Alguna cosa ha fallat. Torna-ho a provar.',
  },
  en: {
    title: 'Device registration', subtitle: 'Add this computer to the inventory, or update its details.',
    server: 'Inventory address', server_ph: 'inventory.company.com', email: 'Email or username', password: 'Password',
    continue: 'Continue', account_hint: 'Use a technician or administrator account of the inventory. The password is not saved.',
    trust_title: "The server's own certificate",
    trust_text: "This server uses its own certificate, which this computer does not recognise. Only trust it if it is your company's inventory.",
    fp_ca: 'SHA-256 fingerprint of its certificate authority', fp_cert: 'SHA-256 fingerprint of the certificate',
    trust_hint: 'Whoever installed the inventory can confirm the fingerprint. It will only be trusted for this address, and remembered.',
    cancel: 'Cancel', trust: 'Trust and continue',
    signed_as: 'Signed in as {name}', review_title: "This computer's details", reading: "Reading the computer's details…",
    serial_manual: 'Serial number (this computer does not report it)',
    new_device: 'This computer is not in the inventory. It will be added with these details:',
    name_is_tag: 'This computer’s name, {name}, will be its asset tag.',
    name_invalid: 'This inventory uses each computer’s name as its asset tag, but “{name}” is not a valid tag. Rename it to {tag}.',
    name_used: 'This inventory uses each computer’s name as its asset tag, but another device already has {name}{used}. Rename it to {tag}.',
    name_differs: 'This inventory uses each computer’s name as its asset tag: this computer is called {name}, but its tag is {tag}. Rename it to {tag}.',
    rename_btn: 'Rename to {tag}', name_settings: 'Open system settings',
    renamed: 'Renamed to {tag}.', renamed_restart: 'Renamed to {tag}. Restart the computer for the new name to apply.',
    rename_cancelled: 'The name was not changed: the permission request was cancelled.', rename_failed: 'Could not rename the computer: {detail}',
    no_settings: 'The system settings could not be found. Rename the computer by hand.', bad_name: '{tag} cannot be a computer name. Rename it by hand.',
    existing: 'This computer is already in the inventory as {tag}. Its hardware and software details will be updated; its status, location, assignment and everything else stay as they are.',
    tag: 'Asset tag', type: 'Type', location: 'Location',
    add: 'Add to the inventory', update: 'Update in the inventory', sign_out: 'Change account',
    done_added: 'Added as {tag}', done_updated: '{tag} updated', done_text: 'You can close this window now.',
    open: 'View in the inventory', close: 'Close', version: 'Version {v}',
    f_serial: 'Serial number', f_name: 'Computer name', f_model: 'Model', f_cpu: 'Processor', f_ram: 'Memory',
    f_disks: 'Storage', f_gpus: 'Graphics', f_screen: 'Screen', f_battery: 'Battery health', f_os: 'System',
    f_enc: 'Disk encryption', f_domain: 'Joined to a domain', f_nics: 'Network', yes: 'Yes', no: 'No', cores: '{n} cores',
    w_serial_missing: 'This computer does not report its serial number. Type it below (it is usually on a label on the computer).',
    w_serial_needs_admin: 'To read the serial number and memory modules, open the app as administrator (sudo).',
    w_battery_needs_admin: 'Battery health can only be read when the app is opened as administrator.',
    w_windows_tools_failed: 'Windows did not answer the hardware queries (PowerShell). Some details are missing.',
    w_other: 'Some details could not be read.',
    e_login_err: 'Wrong email, username or password.', e_login_off: 'This account is deactivated.',
    e_login_locked: 'Too many failed attempts. Wait a few minutes and try again.',
    e_err_must_change: 'This account must change its password on the inventory website before using it here.',
    e_err_forbidden: 'This account can only view the inventory. A technician or administrator account is needed.',
    e_err_auth: 'The session expired. Sign in again.', e_err_dup: 'Another device already uses that asset tag or its identification number.',
    e_err_required: 'The inventory is missing something it needs (a serial number, a device type or a location).',
    e_maintenance: 'The inventory is being restored or updated. Try again in a few minutes.',
    e_not_inventory: 'That address does not look like an inventory.', e_err_invalid: 'The server did not accept the details.',
    e_network: 'Could not connect to the server. Check the address and the network.',
    e_address: 'That address is not valid. It must start with https://', e_fill: 'Fill in the address, email and password.',
    e_serial: "Type the computer's serial number.", e_other: 'Something went wrong. Try again.',
  },
};

const tauri = window.__TAURI__;
const call = (cmd, args) => tauri.core.invoke(cmd, args || {});
const $ = (id) => document.getElementById(id);

let lang = 'es';
try { lang = localStorage.getItem('lang') || ''; } catch (e) { lang = ''; }
if (!I18N[lang]) {
  const nav = (navigator.language || 'es').toLowerCase();
  lang = nav.indexOf('ca') === 0 ? 'ca' : nav.indexOf('es') === 0 ? 'es' : nav.indexOf('en') === 0 ? 'en' : 'es';
}

function t(key, vars) {
  let text = I18N[lang][key];
  if (text === undefined) text = I18N.es[key] !== undefined ? I18N.es[key] : key;
  Object.keys(vars || {}).forEach((k) => { text = text.split('{' + k + '}').join(vars[k]); });
  return text;
}

const state = { start: null, facts: null, factsPromise: null, server: '', offer: null, user: null, options: null, result: null, step: 'connect' };

function show(step) {
  state.step = step;
  ['Connect', 'Trust', 'Review', 'Done'].forEach((s) => { $('step' + s).hidden = s.toLowerCase() !== step; });
  const focus = { connect: firstEmpty(), trust: $('trustBtn'), review: null, done: $('closeBtn') }[step];
  if (focus) setTimeout(() => focus.focus(), 0);
}

function firstEmpty() {
  return ['server', 'email', 'password'].map($).filter((el) => !el.value.trim())[0] || $('connectBtn');
}

function message(problem) {
  if (!problem || typeof problem !== 'object') return t('e_other');
  if (problem.kind === 'server') return I18N.es['e_' + problem.code] ? t('e_' + problem.code) : t('e_other');
  if (problem.kind === 'network') return t('e_network');
  if (problem.kind === 'badAddress') return t('e_address');
  return t('e_other');
}

function setError(id, text) {
  const el = $(id);
  el.textContent = text || '';
  el.hidden = !text;
}

function busy(button, on) {
  button.disabled = on;
  button.classList.toggle('busy', on);
}

function applyLanguage() {
  document.documentElement.lang = lang;
  document.title = t('title');
  document.querySelectorAll('[data-t]').forEach((el) => { el.textContent = t(el.dataset.t); });
  document.querySelectorAll('[data-ph]').forEach((el) => { el.placeholder = t(el.dataset.ph); });
  $('lang').value = lang;
  if (state.start) $('version').textContent = t('version', { v: state.start.version });
  if (state.step === 'trust') renderTrust();
  if (state.step === 'review') renderReview();
  if (state.step === 'done') renderDone();
}

// ---------- 1. connect and sign in

async function signIn() {
  const server = $('server').value.trim();
  const email = $('email').value.trim();
  const password = $('password').value;
  if (!server || !email || !password) { setError('connectError', t('e_fill')); firstEmpty().focus(); return; }
  setError('connectError', '');
  busy($('connectBtn'), true);
  try {
    try {
      state.server = await call('connect', { server: server });
    } catch (problem) {
      if (problem && problem.kind === 'untrusted') {
        state.offer = problem.offer;
        renderTrust();
        show('trust');
        return;
      }
      throw problem;
    }
    await finishSignIn();
  } catch (problem) {
    setError('connectError', message(problem));
  } finally {
    busy($('connectBtn'), false);
  }
}

async function finishSignIn() {
  state.user = await call('sign_in', { email: $('email').value.trim(), password: $('password').value });
  $('password').value = '';
  state.options = null;
  renderReview();
  show('review');
  await loadOptions();
}

// ---------- 2. trust the server's own certificate

function renderTrust() {
  if (!state.offer) return;
  $('fpLabel').textContent = t(state.offer.ca ? 'fp_ca' : 'fp_cert');
  // four lines of eight pairs, easier to compare than one long line
  const pairs = state.offer.fingerprint.split(':');
  const lines = [];
  for (let i = 0; i < pairs.length; i += 8) lines.push(pairs.slice(i, i + 8).join(':'));
  $('fingerprint').textContent = lines.join('\n');
}

async function trust() {
  setError('trustError', '');
  busy($('trustBtn'), true);
  try {
    try {
      state.server = await call('trust', { server: $('server').value.trim(), fingerprint: state.offer.fingerprint });
    } catch (problem) {
      if (problem && problem.kind === 'untrusted') { state.offer = problem.offer; renderTrust(); } // it changed meanwhile
      throw problem;
    }
    await finishSignIn();
  } catch (problem) {
    if (problem && (problem.kind === 'untrusted' || problem.kind === 'network')) setError('trustError', message(problem));
    else { show('connect'); setError('connectError', message(problem)); }
  } finally {
    busy($('trustBtn'), false);
  }
}

// ---------- 3. review

function serial() {
  return (state.facts && state.facts.serial) || $('serialInput').value.trim();
}

async function loadOptions() {
  await state.factsPromise;
  try {
    const facts = state.facts || {};
    setError('nameError', '');
    state.options = await call('options', { serial: serial(), hostname: facts.hostname || '' });
    setError('sendError', '');
  } catch (problem) {
    setError('sendError', message(problem));
  }
  renderReview();
}

function name(item) {
  if (!item) return '';
  if (typeof item.nombre === 'string') return item.nombre;
  return item.nombre[lang] || item.nombre.es || item.nombre.en || '';
}

function yesNo(value) {
  return value === 'yes' ? t('yes') : value === 'no' ? t('no') : '';
}

function number(n) {
  return typeof n === 'number' ? n.toLocaleString(lang, { maximumFractionDigits: 2 }) : '';
}

function factRows(f) {
  const rows = [];
  const add = (key, value) => { if (value && String(value).trim()) rows.push([t(key), String(value).trim()]); };
  add('f_serial', f.serial);
  add('f_name', f.hostname);
  add('f_model', [f.manufacturer, f.model].filter(Boolean).join(' '));
  if (f.cpu) add('f_cpu', [f.cpu.name, f.cpu.cores ? '· ' + t('cores', { n: f.cpu.cores }) : ''].filter(Boolean).join(' '));
  if (f.ram && f.ram.gb) add('f_ram', [number(f.ram.gb) + ' GB', f.ram.type, f.ram.mts ? number(f.ram.mts) + ' MT/s' : ''].filter(Boolean).join(' '));
  add('f_disks', (f.disks || []).map((d) => [d.gb >= 1000 ? number(Math.round(d.gb / 100) / 10) + ' TB' : number(d.gb) + ' GB', d.type, d.connector].filter(Boolean).join(' ')).join(', '));
  add('f_gpus', (f.gpus || []).map((g) => g.name + (g.vramGb ? ' (' + number(g.vramGb) + ' GB)' : '')).join(', '));
  if (f.screenInches) add('f_screen', number(f.screenInches) + '"');
  if (f.battery) add('f_battery', number(f.battery.healthPct) + ' %');
  if (f.os) {
    const family = { mac: 'macOS', win: 'Windows', linux: 'Linux' }[f.os.family] || f.os.family;
    add('f_os', [f.os.distro || family, f.os.version, f.os.edition].filter(Boolean).join(' '));
  }
  add('f_enc', yesNo(f.encryption));
  add('f_domain', yesNo(f.domainJoined));
  add('f_nics', (f.nics || []).map((n) => [
    n.type === 'wifi' ? 'Wi-Fi' + (n.wifiGen ? ' ' + n.wifiGen : '') : 'Ethernet',
    n.model,
    n.btVer ? 'Bluetooth ' + n.btVer : '',
  ].filter(Boolean).join(' · ')).join(', '));
  return rows;
}

function renderReview() {
  $('signedAs').textContent = state.user ? t('signed_as', { name: state.user.nombre }) : '';
  const f = state.facts;
  $('loading').hidden = !!f;
  const dl = $('facts');
  dl.textContent = '';
  if (f) {
    factRows(f).forEach((row) => {
      const dt = document.createElement('dt'); dt.textContent = row[0];
      const dd = document.createElement('dd'); dd.textContent = row[1];
      dl.appendChild(dt); dl.appendChild(dd);
    });
  }
  const ul = $('warnings');
  ul.textContent = '';
  ((f && f.warnings) || []).forEach((w) => {
    const li = document.createElement('li');
    li.textContent = I18N.es['w_' + w] ? t('w_' + w) : t('w_other');
    ul.appendChild(li);
  });
  $('serialBox').hidden = !f || !!f.serial;

  const o = state.options;
  const existing = o && o.existing;
  $('existing').hidden = !existing;
  if (existing) $('existing').textContent = t('existing', { tag: existing.tag });
  $('newFields').hidden = !o || !!existing;
  if (o && !existing) fillNewFields(o, f);
  renderName();
  $('sendBtn').textContent = t(existing ? 'update' : 'add');
  $('sendBtn').disabled = !f || !o;
}

function fillNewFields(o, f) {
  const typeSel = $('type');
  const locSel = $('location');
  const keepType = typeSel.value;
  const keepLoc = locSel.value;
  typeSel.textContent = '';
  (o.types || []).forEach((ty) => typeSel.appendChild(new Option(name(ty), ty.id)));
  locSel.textContent = '';
  (o.locations || []).forEach((l) => locSel.appendChild(new Option(name(l), l.id)));
  typeSel.value = keepType || guessType(o.types || [], f ? f.chassis : '', f ? f.model : '', f ? f.manufacturer : '');
  if (!typeSel.value && typeSel.options.length) typeSel.selectedIndex = 0;
  if (!$('tag').dataset.filled) {
    // with names as tags, a computer already named with a valid, free tag keeps it
    const h = o.hostnameIsTag && o.hostnameTag;
    $('tag').value = h && h.valid && !h.usedBy ? h.tag : tagFor(o, typeSel.value);
    $('tag').dataset.filled = '1'; $('tag').dataset.auto = $('tag').value;
  }
  const storage = (o.locations || []).filter((l) => l.tipo === 'storage')[0];
  locSel.value = keepLoc || (storage ? storage.id : '');
  if (!locSel.value && locSel.options.length) locSel.selectedIndex = 0;
}

// each type numbers its own tags (Z + LAP + 001); older inventories give one suggestion for all
function tagFor(o, typeId) {
  const ty = (o.types || []).filter((x) => x.id === typeId)[0];
  return (ty && ty.nextTag) || o.nextTag || '';
}

// the same choice the server makes when no type is given (inventory/agent.py): a type whose words to match
// (set in the inventory's Settings > Device types) appear in the maker or model, else one named there ("MacBook
// Air" is a MacBook), else one set to take every computer of this shape, then of any shape (each maybe only for
// Apple computers or all but them), else one that fits the shape. A longer word wins.
function guessType(types, chassis, model, manufacturer) {
  const text = ((manufacturer || '') + ' ' + (model || '')).toLowerCase();
  const esc = (w) => w.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  const longest = (words) => Math.max(0, ...words.filter((w) => w && new RegExp('(^|[^a-z])' + esc(w) + '($|[^a-z])').test(text)).map((w) => w.length));
  const names = (ty) => Object.values(ty.nombre && typeof ty.nombre === 'object' ? ty.nombre : { es: ty.nombre }).map((n) => String(n || '').toLowerCase());
  const matches = (ty) => (ty.match || []).map((w) => String(w).toLowerCase());
  for (const words of [matches, names]) {
    let best = null;
    types.forEach((ty) => { const n = longest(words(ty)); if (n && (!best || n > best.n)) best = { id: ty.id, n: n }; });
    if (best) return best.id;
  }
  const apple = /apple/.test((manufacturer || '').toLowerCase()) || /^\s*(imac|macbook|mac(?![a-z]))/.test((model || '').toLowerCase());
  // which makers a type's rule covers: "only" Apple, "except" Apple, or all (inventories before 1.6.0 sent autoNotMac)
  const makers = (ty) => ty.autoApple || (ty.autoNotMac ? 'except' : '');
  const allowed = types.filter((ty) => makers(ty) !== (apple ? 'except' : 'only'));
  const ruled = (chassis && allowed.filter((ty) => ty.autoChassis === chassis)[0]) || allowed.filter((ty) => ty.autoChassis === 'any')[0];
  if (ruled) return ruled.id;
  const wanted = {
    laptop: (ty) => ty.hasBattery,
    all_in_one: (ty) => ty.hasScreen && !ty.hasBattery,
    desktop: (ty) => !ty.hasScreen && !ty.hasBattery && !ty.usesName,
  }[chassis];
  const found = wanted ? allowed.filter(wanted)[0] : null;
  return found ? found.id : '';
}

async function send() {
  const f = state.facts;
  if (!serial()) { setError('sendError', t('e_serial')); $('serialInput').focus(); return; }
  setError('sendError', '');
  busy($('sendBtn'), true);
  try {
    const facts = Object.assign({}, f, { serial: serial() });
    delete facts.warnings;
    const body = { facts: facts };
    if (state.options && !state.options.existing) {
      if ($('tag').value.trim()) body.tag = $('tag').value.trim();
      if ($('type').value) body.typeId = $('type').value;
      if ($('location').value) body.loc = $('location').value;
    }
    state.result = await call('send', { body: body });
    renderDone();
    show('done');
  } catch (problem) {
    setError('sendError', message(problem));
    if (problem && problem.kind === 'server' && problem.code === 'err_auth') signOut();
  } finally {
    busy($('sendBtn'), false);
  }
}

async function signOut() {
  await call('sign_out').catch(() => {});
  state.user = null;
  state.options = null;
  $('tag').dataset.filled = '';
  show('connect');
}

// ---------- the computer's name, when the inventory uses names as asset tags

// the tag this computer should be named after: its tag in the inventory, or the one about to be given
function wantedName() {
  const o = state.options;
  return ((o.existing ? o.existing.tag : $('tag').value) || '').trim();
}

function renderName() {
  const o = state.options, box = $('nameBox');
  const h = o && o.hostnameIsTag && o.hostnameTag;
  box.hidden = !h;
  if (!h) return;
  const want = wantedName();
  const same = want && want.toLowerCase() === h.name.toLowerCase();
  let text;
  if (same && h.valid && !h.usedBy) text = o.existing ? '' : t('name_is_tag', { name: h.name });
  else if (!h.valid) text = t('name_invalid', { name: h.name, tag: want });
  else if (h.usedBy && !(o.existing && h.matches)) text = t('name_used', { name: h.name, used: h.usedBy.toLowerCase() === h.name.toLowerCase() ? '' : ` (${h.usedBy})`, tag: want });
  else text = t('name_differs', { name: h.name, tag: want });
  box.hidden = !text;
  box.classList.toggle('info', !!same);
  $('nameText').textContent = text;
  $('nameActions').hidden = !!same || !want;
  $('renameBtn').textContent = t('rename_btn', { tag: want });
  $('renameBtn').disabled = !/^[A-Za-z0-9](?:[A-Za-z0-9-]{0,13}[A-Za-z0-9])?$/.test(want);
}

async function renameComputer() {
  const want = wantedName();
  setError('nameError', '');
  busy($('renameBtn'), true);
  try {
    const done = await call('rename_computer', { name: want });
    if (state.facts) state.facts.hostname = want;
    if (done.restartNeeded) {
      // Windows applies the new name after a restart: say so, and send the details as usual meanwhile
      $('nameText').textContent = t('renamed_restart', { tag: want });
      $('nameActions').hidden = true;
      $('nameBox').classList.add('info');
    } else {
      await loadOptions();
    }
  } catch (problem) {
    const code = typeof problem === 'string' ? problem : '';
    setError('nameError', code === 'cancelled' ? t('rename_cancelled') : code === 'bad_name' ? t('bad_name', { tag: want }) : t('rename_failed', { detail: code || message(problem) }));
  } finally {
    busy($('renameBtn'), false);
  }
}

// ---------- 4. done

function renderDone() {
  const r = state.result;
  if (!r) return;
  $('doneTitle').textContent = t(r.created ? 'done_added' : 'done_updated', { tag: r.device.tag });
  $('doneText').textContent = t('done_text');
}

// ---------- start

async function init() {
  applyLanguage();
  state.factsPromise = call('collect_facts').then((facts) => {
    state.facts = facts;
    if (state.step === 'review') renderReview();
    return facts;
  });
  try {
    state.start = await call('start');
    $('server').value = state.start.server || '';
    $('email').value = state.start.email || '';
    $('version').textContent = t('version', { v: state.start.version });
  } catch (e) { /* first run */ }
  show('connect');
}

$('connectForm').addEventListener('submit', (e) => { e.preventDefault(); signIn(); });
$('trustBtn').addEventListener('click', trust);
$('trustCancel').addEventListener('click', () => { state.offer = null; show('connect'); });
$('sendBtn').addEventListener('click', send);
$('backBtn').addEventListener('click', signOut);
$('openBtn').addEventListener('click', () => { if (state.result) call('open_inventory', { url: state.result.url }).catch(() => {}); });
$('closeBtn').addEventListener('click', () => call('quit'));
$('serialInput').addEventListener('change', () => { if (state.user) loadOptions(); });
// the suggested tag follows the type, until someone types their own
$('type').addEventListener('change', () => {
  const tag = $('tag');
  if (state.options && tag.value === tag.dataset.auto) tag.value = tag.dataset.auto = tagFor(state.options, $('type').value);
  if (state.options) renderName();
});
$('tag').addEventListener('input', () => { if (state.options) renderName(); });
$('renameBtn').addEventListener('click', renameComputer);
$('nameSettingsBtn').addEventListener('click', () => call('open_name_settings').catch((e) => setError('nameError', e === 'no_settings' ? t('no_settings') : t('rename_failed', { detail: String(e) }))));
$('lang').addEventListener('change', () => {
  lang = $('lang').value;
  try { localStorage.setItem('lang', lang); } catch (e) { /* not saved */ }
  applyLanguage();
});
document.addEventListener('contextmenu', (e) => { if (!e.target.closest('input, dd, .fp')) e.preventDefault(); });

init();
