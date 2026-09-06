import { useEffect, useRef } from "react";
import { boot } from "./engine.js";
import { PromptEditor } from "./PromptEditor.jsx";

export default function App() {
  const booted = useRef(false);
  useEffect(() => {
    if (booted.current) return;
    booted.current = true;
    if (window.I18n) {
      window.I18n.apply(document);
      const langBtn = document.getElementById("lang-btn");
      if (langBtn) window.I18n.bindToggle(langBtn);
    }
    boot();
  }, []);
  return (
    <>
<div id="app">
    <aside id="sidebar">
      <header className="side-head">
        <a className="brand" href="/" aria-label="GGOK">
          <span className="brand-mark" aria-hidden="true">
            <svg className="logo-mark" viewBox="0 0 32 32"><use href="#i-mark"/></svg>
          </span>
          <span className="brand-text">GGOK</span>
        </a>
        <div className="side-head-actions">
          <button type="button" id="search-btn" className="icon-btn" aria-label="搜索" data-i18n-aria="search">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" aria-hidden="true">
              <path d="M17.5 17L20.5 20" strokeLinecap="square"/>
              <circle cx="11.25" cy="10.75" r="7.75"/>
            </svg>
          </button>
          <button type="button" id="collapse-side" className="icon-btn" aria-label="收起侧栏" data-i18n-aria="collapseSidebar">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" aria-hidden="true"><rect x="3.5" y="4" width="17" height="16" rx="4"/><path d="M9 4V20"/></svg>
          </button>
        </div>
      </header>
      <div className="new-chat-item">
        <span className="new-chat-glow" aria-hidden="true" />
        <button type="button" id="new-session" className="side-nav">
          <span className="nav-ico" aria-hidden="true">
            <span className="nav-ico-inner">
              <svg viewBox="0 0 24 24" width="24" height="24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="square" aria-hidden="true">
                <rect x="4" y="4" width="16" height="16" rx="4"/>
                <path d="M12 8v8M8 12h8"/>
              </svg>
            </span>
          </span>
          <span className="whitespace-nowrap" data-i18n="newSession">新会话</span>
        </button>
        <span className="new-chat-kbd-slot" aria-hidden="true">
          <span className="new-chat-kbd">⌘J</span>
        </span>
      </div>
      <div id="tree" className="alpha-mask-y"></div>
      <footer className="side-foot">
        <button type="button" id="ext-btn" className="side-nav">
          <svg viewBox="0 0 24 24" aria-hidden="true"><use href="#i-grid"/></svg>
          <span data-i18n="extNav">插件 · 连接器 · 技能</span>
        </button>
        <div className="foot-quota-row">
          <button type="button" id="quota-btn" className="quota-btn" data-tip="周限额" data-i18n-title="weeklyLimit" aria-haspopup="dialog" aria-expanded="false">
            <span className="quota-ring" aria-hidden="true"></span>
            <span className="quota-meta">
              <span className="quota-track"><span id="quota-fill" className="quota-fill"></span></span>
              <span id="quota-pct" className="quota-pct">—</span>
            </span>
          </button>
          <div id="quota-pop" className="quota-pop" hidden={true} role="dialog" aria-label="周限额" data-i18n-aria="weeklyLimit">
            <div id="quota-pop-body" className="account-body"></div>
            <div id="quota-ver" className="quota-ver">
              <div className="quota-row">
                <span className="quota-row-k" data-i18n="currentVersion">当前版本</span>
                <span id="quota-ver-cur" className="quota-row-v"></span>
              </div>
              <div className="quota-row">
                <span className="quota-row-k" data-i18n="latestVersion">最新版本</span>
                <button type="button" id="quota-ver-latest" className="quota-ver-latest"></button>
              </div>
            </div>
            <form action="/logout" method="post" className="quota-logout-form">
              <button type="submit" id="logout-btn" className="side-logout">
                <svg viewBox="0 0 24 24" aria-hidden="true"><use href="#i-logout"/></svg>
                <span data-i18n="logout">登出</span>
              </button>
            </form>
          </div>
        </div>
      </footer>
    </aside>
    <div id="scrim" className="ui-scrim" hidden={true}></div>
    <section id="main">
      <div id="chatcol">
      <header id="toolbar">
        <div className="toolbar-fade" aria-hidden="true"></div>
        <button type="button" id="open-side" className="icon-btn" data-tip="会话列表" data-i18n-title="sessionList">
          <svg viewBox="0 0 24 24"><use href="#i-menu"/></svg>
        </button>
        <div id="titlebar">
          <div className="title-tools">
            <a id="github-link" className="icon-btn" href="https://github.com/jjdufu/ggok" target="_blank" rel="noopener noreferrer" aria-label="GitHub">
              <svg viewBox="0 0 24 24" aria-hidden="true"><use href="#i-github"/></svg>
            </a>
            <button type="button" id="lang-btn" className="icon-btn lang-btn" aria-label="中文">
              <span className="lang-icon" data-lang-icon="zh">中</span>
              <span className="lang-icon" data-lang-icon="en">EN</span>
            </button>
            <button type="button" id="theme-btn" className="icon-btn theme-btn" aria-label="浅色">
              <svg className="theme-icon" data-theme-icon="light" viewBox="0 0 24 24"><use href="#i-sun"/></svg>
              <svg className="theme-icon" data-theme-icon="dark" viewBox="0 0 24 24"><use href="#i-moon"/></svg>
            </button>
            <div id="actions" hidden={true}>
              <button type="button" id="copy-turn" className="icon-btn" aria-label="复制本轮" data-i18n-aria="copyTurn">
                <svg viewBox="0 0 24 24"><use href="#i-copy"/></svg>
              </button>
              <a id="dl-md" className="icon-btn" href="#" aria-label="下载 md" data-i18n-aria="downloadMd">
                <svg viewBox="0 0 24 24"><use href="#i-download"/></svg>
              </a>
              <div id="info-pop" hidden={true} role="dialog" aria-label="会话信息" data-i18n-aria="sessionInfo">
                <div className="usage-pop-title" id="info-title">Session</div>
                <div id="info-body" className="info-body"></div>
              </div>
            </div>
            <button type="button" id="usage-toggle" className="icon-btn" aria-pressed="false" aria-expanded="false" aria-label="状态" data-i18n-aria="status">
              <svg viewBox="0 0 24 24" aria-hidden="true"><use href="#i-usage"/></svg>
            </button>
            <button type="button" id="ws-toggle" className="icon-btn" aria-pressed="false" aria-expanded="false" aria-label="工作区" data-i18n-aria="workspace">
              <svg viewBox="0 0 24 24" aria-hidden="true"><use href="#i-folder"/></svg>
            </button>
          </div>
        </div>
      </header>
      <div id="stage">
        <div id="empty-state">
          <h2>GGOK</h2>
        </div>
        <div id="timeline"></div>
        <footer id="composer">
          <div className="composer-fade" aria-hidden="true"></div>
          <div className="composer-wrap">
            <div id="occupy-banner" className="occupy-banner" hidden={true}></div>
            <div id="queue" hidden={true}></div>
            <div id="slash-menu" hidden={true}></div>
            <div id="at-menu" hidden={true}></div>
            <div id="ctx-bar" className="ctx-bar composer-ctx-bar" hidden={true}>
              <div className="ctx-track">
                <div id="ctx-fill" className="ctx-fill"></div>
                <span id="ctx-label" className="ctx-label"></span>
              </div>
            </div>
            <div className="composer-inner">
              <div id="chips" hidden={true}></div>
              <div className="prompt-shell">
                <PromptEditor />
                <div id="prompt-ph" aria-hidden="true"></div>
              </div>
              <div className="composer-bar">
                <button type="button" id="attach-btn" className="icon-btn attach-btn" data-tip="附件" data-i18n-title="attach">
                  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" aria-hidden="true">
                    <path d="M6 12H18M12 6V18" strokeLinecap="square"/>
                  </svg>
                </button>
                <input id="file-input" type="file" multiple={true} hidden={true} />
                <button type="button" id="dir-btn" className="composer-btn pending" data-tip="选择工作目录" data-i18n-title="pickCwd">
                  <svg viewBox="0 0 24 24" aria-hidden="true"><use href="#i-folder"/></svg>
                  <span id="dir-label" hidden={true}><span className="dir-path-text"></span></span>
                </button>
                <span className="composer-spacer"></span>
                <div className="model-wrap">
                  <button type="button" id="model-btn" className="model-btn" data-tip="模型" data-i18n-title="model">
                    <span id="model-label">4.6</span>
                    <svg viewBox="0 0 16 16" aria-hidden="true"><use href="#i-chevron"/></svg>
                  </button>
                  <div id="model-menu" hidden={true}></div>
                </div>
                <button type="button" id="send-btn" className="submit-orb" data-testid="chat-submit" data-tip="发送" data-i18n-title="send">
                  <div className="send-face">
                    <span id="send-icon" className="t-icon-swap" data-state="a">
                      <svg className="t-icon" data-icon="a" width="20" height="20" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
                        <path d="M6 11L12 5M12 5L18 11M12 5V19" stroke="currentColor" strokeLinecap="square"/>
                      </svg>
                      <svg className="t-icon" data-icon="b" width="20" height="20" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
                        <rect x="7" y="7" width="10" height="10" fill="currentColor"/>
                      </svg>
                    </span>
                  </div>
                </button>
              </div>
            </div>
          </div>
        </footer>
      </div>
      </div>
      <div id="drawer-scrim" className="ui-scrim" hidden={true}></div>
      <aside id="drawer" hidden={true}>
        <header className="drawer-head">
          <div className="drawer-title" id="drawer-title" data-i18n="process">过程</div>
          <button type="button" id="drawer-close" className="icon-btn drawer-x" data-tip="关闭" data-i18n-title="close">
            <svg viewBox="0 0 24 24"><use href="#i-x"/></svg>
          </button>
        </header>
        <div id="drawer-body"></div>
        <div id="drawer-status" hidden={true}>
          <div className="usage-pop-title" data-i18n="sessionUsage">会话用量</div>
          <div className="usage-pop-sub" data-i18n="sinceStart">自启动或上次恢复起</div>
          <div id="usage-body" className="usage-body"></div>
          <div className="host-head" data-i18n="host">主机</div>
          <div id="host-body" className="host-body"></div>
        </div>
        <div id="drawer-files" hidden={true}>
          <div className="ws-toolbar">
            <button type="button" id="ws-up" className="ws-tool-btn" data-i18n="wsUp">上级</button>
            <button type="button" id="ws-refresh" className="ws-tool-btn" data-i18n="wsRefresh">刷新</button>
            <span className="ws-toolbar-spacer"></span>
            <button type="button" id="ws-pack" className="composer-btn" data-i18n="wsPack">打包工作目录</button>
          </div>
          <div id="ws-path" className="ws-path"></div>
          <div id="ws-list" className="ws-list"></div>
        </div>
      </aside>
    </section>
  </div>

  <div id="ext-scrim" className="ui-scrim" hidden={true}></div>
  <div id="ext-modal" className="ui-dialog" hidden={true} role="dialog" aria-modal="true">
    <header className="ext-head">
      <div className="ext-tabs" id="ext-tabs"></div>
      <div className="ext-tools">
        <label className="ext-search-wrap">
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg" aria-hidden="true">
            <path d="M17.5 17L20.5 20" stroke="currentColor" strokeLinecap="square"></path>
            <circle cx="11.25" cy="10.75" r="7.75" stroke="currentColor"></circle>
          </svg>
          <input id="ext-search" type="text" autoComplete="off" spellCheck="false" />
        </label>
        <div className="ext-cta-wrap">
          <button type="button" id="ext-cta" className="ext-cta"></button>
          <div id="ext-skill-menu" hidden={true} role="menu">
            <button type="button" className="ext-skill-item" role="menuitem" data-skill-act="write">
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg"><path d="M18.25 5.75C16.8693 4.36929 14.6307 4.36929 13.25 5.75L10.125 8.875L5.52404 13.476C4.86236 14.1376 4.45361 15.0104 4.36889 15.9423L4 20.0001L8.0578 19.6311C8.98967 19.5464 9.86234 19.1377 10.524 18.476L18.25 10.75C19.6307 9.36929 19.6307 7.13071 18.25 5.75V5.75Z" stroke="currentColor"></path><path d="M12.5 7.5L16.5 11.5" stroke="currentColor"></path></svg>
              <span data-i18n="writeSkill">Write skill manually</span>
            </button>
            <button type="button" className="ext-skill-item" role="menuitem" data-skill-act="upload">
              <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"></path><polyline points="17 8 12 3 7 8"></polyline><line x1="12" x2="12" y1="3" y2="15"></line></svg>
              <span data-i18n="uploadSkill">Upload skill file</span>
            </button>
            <button type="button" className="ext-skill-item" role="menuitem" data-skill-act="ai">
              <svg width="16" height="16" viewBox="0 0 24 24" aria-hidden="true"><use href="#i-spark"/></svg>
              <span data-i18n="createSkillAi">Create skill in chat</span>
            </button>
          </div>
          <input id="ext-skill-file" type="file" accept=".md,.markdown,.zip,.skill" hidden={true} />
        </div>
      </div>
    </header>
    <div id="ext-grid" className="ext-grid"></div>
    <div id="ext-detail" className="ext-detail" hidden={true}></div>
    <form id="ext-add" className="ext-add" hidden={true}></form>
  </div>
  <div id="finder-scrim" className="ui-scrim" hidden={true}></div>
  <div id="finder" className="ui-dialog" hidden={true} role="dialog" aria-modal="true">
    <label className="finder-search">
      <input id="finder-q" type="search" autoComplete="off" spellCheck="false" data-i18n-placeholder="search" />
      <svg viewBox="0 0 24 24" aria-hidden="true"><use href="#i-search"/></svg>
    </label>
    <div className="finder-body">
      <div id="finder-list" className="finder-list"></div>
      <div id="finder-preview" className="finder-preview"></div>
    </div>
    <footer className="finder-foot">
      <button type="button" id="finder-resize" className="finder-resize">
        <svg viewBox="0 0 24 24"><use href="#i-collapse"/></svg>
      </button>
      <div className="finder-foot-keys">
        <div className="finder-keys">
          <span data-i18n="finderGo">前往</span>
          <kbd>⏎</kbd>
        </div>
        <div className="finder-keys">
          <span data-i18n="finderEdit">编辑</span>
          <kbd id="finder-kbd-edit">⌘⇧E</kbd>
        </div>
        <div className="finder-keys">
          <span data-i18n="finderDelete">删除</span>
          <kbd id="finder-kbd-delete">⌘⇧D</kbd>
        </div>
      </div>
    </footer>
  </div>

  <div id="dir-scrim" className="ui-scrim" hidden={true}></div>
  <div id="dir-modal" className="ui-dialog" hidden={true} role="dialog" aria-modal="true" aria-labelledby="dir-modal-title" tabIndex={-1}>
    <header className="dir-modal-head">
      <div id="dir-modal-title" className="dir-modal-title" data-i18n="pickCwd">选择工作目录</div>
      <button type="button" id="dir-modal-close" className="icon-btn" data-tip="关闭" data-i18n-title="close">
        <svg viewBox="0 0 24 24"><use href="#i-x"/></svg>
      </button>
    </header>
    <div id="dir-modal-path" className="dir-modal-path"></div>
    <div id="dir-modal-list" className="dir-modal-list"></div>
    <footer className="dir-modal-foot">
      <button type="button" id="dir-modal-choose" className="dir-choose-btn" disabled={true} data-i18n="chooseThisDir">选择此目录</button>
    </footer>
  </div>

  <div id="sess-menu" className="sess-menu" hidden={true} role="menu"></div>
  <div id="app-confirm" className="ext-confirm sess-confirm ui-scrim" hidden={true}>
    <div className="ext-confirm-card" role="dialog" aria-modal="true">
      <div id="confirm-title" className="ext-confirm-title"></div>
      <div id="confirm-body" className="ext-confirm-body"></div>
      <div className="ext-confirm-actions">
        <button type="button" id="confirm-cancel" className="ext-cancel-btn"></button>
        <button type="button" id="confirm-ok" className="mcp-add-btn"></button>
      </div>
    </div>
  </div>
    </>
  );
}
