/* Duty Scheduler — menu.js
   Slide-out nav panel. Page-specific links by role.
   IIFE: injects CSS, HTML, JS. Sets window.ds_auth. */
(function() {
    var css = document.createElement('style');
    css.textContent = [
        '.menu-btn{position:fixed;top:12px;right:12px;background:var(--card);border:1px solid var(--accent);color:var(--accent);width:44px;height:44px;border-radius:50%;cursor:pointer;font-size:22px;z-index:9999;display:flex;align-items:center;justify-content:center;transition:background .2s}',
        '.menu-btn:hover{background:rgba(56,189,248,.15)}',
        '.menu-overlay{position:fixed;inset:0;background:rgba(0,0,0,.5);z-index:9998;opacity:0;visibility:hidden;transition:.3s}',
        '.menu-overlay.active{opacity:1;visibility:visible}',
        '.menu-panel{position:fixed;top:0;right:-300px;width:280px;height:100%;background:var(--card);z-index:9999;transition:.3s;padding:80px 16px 20px;overflow-y:auto;border-left:1px solid var(--border)}',
        '.menu-panel.active{right:0}',
        '.menu-panel .mp-title{font-size:18px;font-weight:700;color:var(--accent);margin-bottom:20px}',
        '.menu-panel .mp-role{font-size:.78rem;color:var(--muted);margin-bottom:16px;padding-bottom:12px;border-bottom:1px solid var(--border)}',
        '.menu-panel a,.menu-panel button.mp-link{display:block;padding:12px 14px;color:var(--text);text-decoration:none;border-radius:8px;margin-bottom:6px;font-size:.88rem;cursor:pointer;border:none;background:none;text-align:left;width:100%;font-family:inherit}',
        '.menu-panel a:hover,.menu-panel button.mp-link:hover{background:rgba(56,189,248,.1);color:var(--accent)}',
        '.menu-panel a.active{background:rgba(56,189,248,.15);color:var(--accent);font-weight:600}',
        '.menu-panel .mp-sep{height:1px;background:var(--border);margin:10px 0}',
    ].join('\n');
    document.head.appendChild(css);

    var overlay = document.createElement('div');
    overlay.className = 'menu-overlay';
    overlay.id = 'menuOverlay';
    overlay.onclick = function() { window.toggleMenu(); };
    document.body.appendChild(overlay);

    var panel = document.createElement('div');
    panel.className = 'menu-panel';
    panel.id = 'menuPanel';
    document.body.appendChild(panel);

    var btn = document.createElement('button');
    btn.className = 'menu-btn';
    btn.innerHTML = '☰';
    btn.onclick = function() { window.toggleMenu(); };
    document.body.appendChild(btn);

    window.toggleMenu = function() {
        overlay.classList.toggle('active');
        panel.classList.toggle('active');
    };

    document.addEventListener('keydown', function(e) {
        if (e.key === 'Escape' && overlay.classList.contains('active')) {
            window.toggleMenu();
        }
    });

    var token = localStorage.getItem('ds_token');
    if (!token) {
        window.location.href = '/activate.html';
        return;
    }

    fetch('/api/auth/me?token=' + encodeURIComponent(token))
        .then(function(r) {
            if (!r.ok) throw new Error('auth failed');
            return r.json();
        })
        .then(function(info) {
            window.ds_auth = info;
            render(info);
        })
        .catch(function() {
            localStorage.removeItem('ds_token');
            window.location.href = '/activate.html';
        });

    function render(info) {
        var page = location.pathname;
        var h = '';
        var roleLabel = info.role === 'superadmin' ? 'Суперадмин' : info.role === 'admin' ? 'Админ' : 'Житель';
        var bname = info.building_name || '';
        h += '<div class="mp-title">Дежурства</div>';
        h += '<div class="mp-role">' + esc(bname) + ' · ' + esc(info.name || roleLabel) + '</div>';

        if (info.role === 'superadmin') {
            h += link('/', '🏢 Здания', page === '/');
            h += link('/admin.html', '📋 Управление', page === '/admin.html');
            h += link('/view.html', '👁 Просмотр', page === '/view.html');
        } else if (info.role === 'admin') {
            h += link('/admin.html', '📋 Управление', page === '/admin.html');
            h += link('/view.html', '👁 Просмотр', page === '/view.html');
        } else {
            h += link('/view.html', '📋 Расписание', page === '/view.html');
        }

        h += '<div class="mp-sep"></div>';
        h += '<button class="mp-link" style="color:var(--red)" onclick="window.dsLogout()">🚪 Выход</button>';

        panel.innerHTML = h;
    }

    function link(href, label, active) {
        return '<a href="' + href + '" class="' + (active ? 'active' : '') + '">' + label + '</a>';
    }

    window.dsLogout = function() {
        localStorage.removeItem('ds_token');
        window.location.href = '/activate.html';
    };

    function esc(s) {
        var d = document.createElement('div');
        d.textContent = s || '';
        return d.innerHTML;
    }
})();
