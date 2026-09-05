(() => {
  const KEY = "ggok-lang";
  const dict = {
    zh: {
      newSession: "新会话",
      themeLight: "浅色",
      themeDark: "深色",
      collapseSidebar: "收起侧栏",
      expandSidebar: "展开侧栏",
      searchSessions: "搜索会话",
      search: "搜索",
      finderOps: "操作",
      finderNew: "创建新聊天",
      finderToday: "今天",
      finderYesterday: "昨天",
      finderWeek: "最近7天",
      finderOlder: "更早",
      finderPreview: "选择要预览的对话",
      finderGo: "前往",
      finderCurrent: "当前",
      finderResize: "缩放",
      finderHidePreview: "隐藏对话预览",
      finderShowPreview: "显示对话预览",
      finderEdit: "编辑",
      finderDelete: "删除",
      rename: "重命名",
      delete: "删除",
      pin: "置顶",
      unpin: "取消置顶",
      pinnedGroup: "置顶",
      sessionMenu: "会话操作",
      renamePlaceholder: "会话标题",
      deleteChatTitle: "删除此对话？",
      deleteChatBody: "此操作无法撤销。",
      confirmDelete: "确认删除",
      contextUsage: "上下文占用",
      sessionList: "会话列表",
      status: "状态",
      workspace: "工作区",
      wsUp: "上级",
      wsRefresh: "刷新",
      wsPack: "打包工作目录",
      wsPacking: "打包中",
      wsDownload: "下载",
      wsAtRef: "连接到对话",
      wsDeleteTitle: "删除「{name}」？",
      wsDeleteBody: "此操作无法撤销。",
      wsEmpty: "此目录为空",
      wsTruncated: "条目过多，仅显示前 2000 项",
      wsSkipHint: "打包时跳过 .git、node_modules、target、__pycache__",
      copyTurn: "复制本轮",
      downloadMd: "下载 md",
      sessionInfo: "会话信息",
      logout: "登出",
      promptPlaceholder: "输入 / 使用斜杠命令",
      promptPh0: "输入 / 使用斜杠命令",
      promptPh1: "输入 / 使用斜杠命令",
      promptPh2: "输入 @ 引用工作区文件",
      promptPh3: "把文件拖进对话",
      attach: "附件",
      processing: "处理中",
      pickCwd: "选择工作目录",
      model: "模型",
      send: "发送",
      stop: "停止",
      process: "过程",
      close: "关闭",
      sessionUsage: "会话用量",
      sinceStart: "自启动或上次恢复起",
      host: "主机",
      langZh: "中文",
      langEn: "English",
      enter: "进入",
      loginTitle: "登录 · GGOK",
      sessionBusy: "会话被终端占用",
      occupyForeign: "终端 grok 占用中",
      occupyObserve: "grok 仍在跑，控制面未连接",
      justNow: "刚刚",
      minutesAgo: "{n} 分钟前",
      hoursAgo: "{n} 小时前",
      daysAgo: "{n} 天前",
      copied: "已复制",
      copyFailed: "复制失败",
      noSessions: "没有会话",
      copy: "复制",
      copyCode: "复制",
      edit: "编辑",
      working: "工作中",
      workedSeconds: "工作了 {n}s",
      itemCount: "{n} 项",
      itemCountOne: "1 项",
      workDone: "工作完成",
      userStopped: "用户终止",
      effortLow: "低",
      effortMedium: "中",
      effortHigh: "高",
      effortXhigh: "极高",
      effortLowDesc: "更快，消耗更少",
      effortMediumDesc: "速度与质量平衡",
      effortHighDesc: "更充分的推理",
      effortXhighDesc: "最充分的推理",
      slashTuiOnly: "WebUI 里没有这个动作，这是 CLI 终端专用命令",
      noModels: "还没有模型列表",
      allowTool: "允许此工具？",
      allow: "允许",
      deny: "拒绝",
      emptySession: "这条会话还没有对话。",
      loading: "加载中…",
      loadFailed: "加载失败：{e}",
      agentError: "出错",
      requestFailed: "请求失败",
      remaining: "剩余",
      left: "剩余",
      noModelCalls: "本会话还没有模型调用。",
      inputTokens: "输入 token",
      outputTokens: "输出 token",
      totalTokens: "总 token",
      modelCalls: "模型调用",
      apiTime: "API 耗时",
      cost: "费用",
      cachedNote: "({n} 缓存)",
      reasoningNote: "({n} 推理)",
      inOut: "{inn} 入 / {out} 出",
      weeklyLimit: "周限额",
      monthlyLimit: "月限额",
      dailyLimit: "日限额",
      usageLimit: "用量限额",
      resetsSoon: "即将重置",
      resetsInDh: "{d} 天 {h} 小时后重置",
      resetsInHm: "{h} 小时 {m} 分钟后重置",
      resetsInM: "{m} 分钟后重置",
      loadingUsage: "正在加载用量…",
      couldntLoadUsage: "无法加载用量",
      plan: "套餐",
      account: "账号",
      usedPct: "{n}% 已用",
      leftPct: "{n}% 剩余",
      used: "已用",
      remaining: "剩余",
      resets: "重置",
      user: "用户",
      hostname: "主机名",
      lanIpv4: "局域网 IPv4",
      wanIpv4: "公网 IPv4",
      cpu: "CPU",
      memory: "内存",
      disk: "磁盘 {path}",
      used: "已用",
      window: "窗口",
      remainingLabel: "剩余",
      input: "输入",
      output: "输出",
      total: "合计",
      cachedLabel: "缓存",
      id: "ID",
      cwd: "工作目录",
      title: "标题",
      emptyQueue: "(空)",
      sendNow: "现在发送",
      recallQueue: "撤回",
      allowedRoots: "起始位置",
      atNoMatch: "没有匹配的文件",
      chooseThisDir: "选择此目录",
      parentDir: "← 上级",
      mcps: "MCP",
      mcpSearch: "搜索服务器",
      mcpRefresh: "刷新",
      mcpEmpty: "还没有 MCP 服务器",
      mcpLocal: "本地",
      mcpDisabled: "已停用",
      mcpEnable: "启用",
      mcpDisable: "停用",
      mcpRemove: "删除",
      mcpConfirmRemove: "确认删除",
      mcpAdd: "添加",
      mcpName: "名称",
      mcpCommand: "命令或 URL",
      mcpArgs: "参数，空格分隔，可选",
      mcpScopeUser: "用户",
      mcpScopeProject: "项目",
      mcpAddNeed: "名称和命令都要填",
      mcpTools: "工具",
      mcpNoTools: "还没有工具",
      mcpHealth: "状态",
      skillFiles: "文件",
      skillLoading: "正在加载技能…",
      skillBinary: "二进制文件",
      plugins: "插件",
      extNav: "插件 · 连接器 · 技能",
      marketplace: "市场",
      connectors: "连接器",
      skills: "技能",
      newConnector: "新连接器",
      newSkill: "新技能",
      writeSkill: "手动编写技能",
      uploadSkill: "上传技能文件",
      createSkillAi: "用对话创建技能",
      personal: "个人",
      quickSkills: "快捷技能",
      quickEmpty: "没有匹配的快捷技能",
      slash_new: "开启新会话并清空当前对话",
      slash_resume: "打开会话列表，从磁盘恢复之前的会话",
      slash_dashboard: "打开 Agent 看板，查看本页的会话",
      slash_compact: "压缩对话历史，腾出上下文空间",
      slash_context: "显示上下文窗口占用情况",
      slash_session_info: "显示会话详情：登录方式、模型、轮次和上下文",
      slash_fork: "从当前进度分出新会话",
      slash_rewind: "回退到更早的一轮，丢掉之后的内容",
      slash_undo: "回退到更早的一轮，丢掉之后的内容",
      slash_copy: "复制最近一次回复的 Markdown",
      slash_export: "把对话导出到文件或剪贴板",
      slash_quit: "退出应用",
      slash_exit: "退出应用",
      slash_home: "离开当前会话，回到欢迎页",
      slash_welcome: "离开当前会话，回到欢迎页",
      slash_delete: "删除当前会话历史",
      slash_rename: "重命名当前会话",
      slash_title: "重命名当前会话",
      slash_model: "切换模型。可填模型 ID 或显示名，推理模型还可加努力程度",
      slash_m: "切换模型。可填模型 ID 或显示名，推理模型还可加努力程度",
      slash_effort: "不换模型，只改当前模型的推理程度",
      slash_always_approve: "切换始终批准权限模式",
      slash_auto: "切换自动批准权限模式",
      slash_multiline: "切换多行输入",
      slash_history: "搜索本会话的历史提示",
      slash_compact_mode: "切换紧凑显示，减少留白、排得更密",
      slash_vim_mode: "切换 vim 风格的回看快捷键",
      slash_edit_prompt: "用外部编辑器编辑提示词",
      slash_minimal: "切换到精简显示模式",
      slash_fullscreen: "切换到全屏显示模式",
      slash_plan: "进入计划模式",
      slash_view_plan: "预览当前保存的计划",
      slash_show_plan: "预览当前保存的计划",
      slash_plan_view: "预览当前保存的计划",
      slash_memory: "浏览和管理已保存的记忆",
      slash_mem: "浏览和管理已保存的记忆",
      slash_flush: "立刻把当前会话要点写入记忆",
      slash_dream: "整理记忆，把会话记录合并成主题",
      slash_remember: "立刻把一条笔记写入记忆",
      slash_hooks: "打开扩展弹窗的 Hooks 页",
      slash_plugins: "打开扩展弹窗的插件页",
      slash_marketplace: "打开扩展弹窗的市场页",
      slash_skills: "打开扩展弹窗的技能页",
      slash_imagine: "根据文字描述生成图片",
      slash_imagine_video: "根据文字或图片描述生成视频",
      slash_loop: "按间隔重复运行一条提示",
      slash_goal: "设置、管理或检查自主目标",
      slash_deep_research: "启动后台研究流程",
      slash_workflow: "启动已保存的工作流，或管理正在跑的",
      slash_workflows: "打开扩展弹窗的工作流页",
      slash_theme: "切换颜色主题",
      slash_t: "切换颜色主题",
      slash_feedback: "反馈问题或建议",
      slash_btw: "给 Agent 发一句旁白，不打断当前任务",
      slash_mcps: "打开 MCP 服务器管理",
      slash_doctor: "检查终端、剪贴板、输入和沙箱等问题",
      slash_release_notes: "查看当前版本的更新说明",
      slash_changelog: "查看当前版本的更新说明",
      slash_docs: "浏览内置指南或打开在线文档",
      slash_howto: "浏览内置指南或打开在线文档",
      slash_guides: "浏览内置指南或打开在线文档",
      slash_tutorial: "打开入门教程",
      slash_import_claude: "导入 Claude 的 ~/.claude 设置",
      slash_config_agents: "打开 Agent 定义管理",
      slash_agents: "打开 Agent 定义管理",
      slash_personas: "创建、编辑和删除人设",
      slash_login: "登录或重新验证",
      slash_logout: "登出并回到登录页",
      slash_usage: "查看用量或管理账单",
      slash_cost: "查看用量或管理账单",
      slash_privacy: "打开编码数据、保留和训练设置",
      slash_settings: "打开设置",
      slash_config: "打开设置",
      slash_preferences: "打开设置",
      slash_prefs: "打开设置",
      slash_timestamps: "开关消息时间戳",
      slash_clear: "开启新会话并清空当前对话",
      slash_agents_dashboard: "打开 Agent 看板，查看本页的会话",
      slash_sessions: "打开 Agent 看板，查看本页的会话",
      slash_status: "显示会话详情：登录方式、模型、轮次和上下文",
      slash_info: "显示会话详情：登录方式、模型、轮次和上下文",
      skillQ_docx: "Word 文档",
      skillQ_docxDesc: "创建、读取和编辑 Word 文档（.docx）",
      skillQ_pdf: "PDF",
      skillQ_pdfDesc: "创建、合并、拆分 PDF，并从中提取内容",
      skillQ_pptx: "演示文稿",
      skillQ_pptxDesc: "创建、读取和编辑 PowerPoint 演示文稿（.pptx）",
      skillQ_create_skill: "技能创建器",
      skillQ_create_skillDesc: "通过对话创建自定义技能",
      skillQ_create_workflow: "创建工作流",
      skillQ_create_workflowDesc: "编写新的多 Agent 工作流",
      skillQ_build_with_ai: "用 AI 构建",
      skillQ_build_with_aiDesc: "在 SpaceXAI 上构建 AI 应用",
      skillQ_code_review: "代码审查",
      skillQ_code_reviewDesc: "严格审查可维护性、抽象质量和过大文件",
      skillQ_design: "设计文档",
      skillQ_designDesc: "撰写设计文档并循环评审，直到达成共识",
      skillQ_execute_plan: "执行计划",
      skillQ_execute_planDesc: "按设计文档里的 PR 计划逐步实现",
      skillQ_implement: "实现",
      skillQ_implementDesc: "实现、审查、修复循环，直到没有问题",
      skillQ_review: "评审",
      skillQ_reviewDesc: "评审本地改动、分支或 GitHub PR",
      skillQ_pr_babysit: "PR 看护",
      skillQ_pr_babysitDesc: "盯 PR、修 CI、处理评审意见和冲突",
      skillQ_imagine: "Imagine 图像",
      skillQ_imagineDesc: "Imagine 图像工具的提示词和流程",
      skillQ_game_asset_core: "游戏素材规范",
      skillQ_game_asset_coreDesc: "游戏素材的引擎就绪默认规则",
      skillQ_game_animation_frames: "游戏动画帧",
      skillQ_game_animation_framesDesc: "面向视频、能循环播放的动画帧",
      skillQ_game_character_consistency: "角色一致性",
      skillQ_game_character_consistencyDesc: "同一角色在每张图里都保持一致",
      skillQ_game_tilesets: "游戏图块",
      skillQ_game_tilesetsDesc: "无缝拼接的地形图块和过渡",
      skillQ_game_ui_icons: "游戏 UI 图标",
      skillQ_game_ui_iconsDesc: "游戏 UI 套件和图标集",
      skillQ_resume_claude: "恢复 Claude",
      skillQ_resume_claudeDesc: "从最近的 Claude Code 会话继续",
      skillQ_resume_codex: "恢复 Codex",
      skillQ_resume_codexDesc: "从最近的 Codex 会话继续",
      skillQ_resume_cursor: "恢复 Cursor",
      skillQ_resume_cursorDesc: "从最近的 Cursor 会话继续",
      skillQ_skill_design_principles: "技能设计原则",
      skillQ_skill_design_principlesDesc: "编写和修改技能时用的简洁原则",
      skillDocuments: "文档",
      skillGame: "游戏",
      skillResume: "会话恢复",
      skillBuiltin: "内置",
      skillName: "名称",
      skillDesc: "描述",
      skillBody: "内容",
      skillCreateNeed: "名称和描述都要填",
      extAdd: "添加",
      extAdded: "已添加",
      extSearch: "搜索...",
      skillEmpty: "还没有技能",
      extBack: "返回",
      extAllSkills: "所有技能",
      extAllMarket: "所有市场",
      extAllPlugins: "所有插件",
      extAllConnectors: "所有连接器",
      extTry: "试试看",
      extTryNow: "现在试试",
      cancel: "取消",
      extTrustTitle: "信任并安装？",
      extTrustBody: "将安装 {name}。需要先信任这个插件。",
      pluginSearch: "搜索插件",
      pluginEmpty: "还没有已装插件",
      marketEmpty: "没有匹配的市场插件",
      pluginInstall: "安装",
      pluginInstalling: "安装中…",
      pluginInstallOk: "已安装 {name}",
      pluginConfirmInstall: "确认信任并安装",
      pluginUninstall: "卸载",
      pluginConfirmUninstall: "确认卸载",
      pluginUpdate: "更新",
      pluginInstalled: "已安装",
      pluginSkills: "{n} 个 skill",
      pluginSourcePh: "git URL / GitHub shorthand / 本地路径",
      pluginInstallPh: "名称、git URL 或 GitHub shorthand",
      pluginAddSource: "添加源",
      pluginRemoveSource: "移除源",
      pluginConfirmRemoveSource: "确认移除源及其插件",
      pluginAddNeed: "填一个源或插件名",
      pickCwdFirst: "先选工作目录",
      runningTool: "正在运行 {name}{preview}",
      ranTools: "已运行 {n} 个 {name}",
      rateLimited: "失败次数过多，请 15 分钟后再试",
      badToken: "token 不正确",
      loginFailed: "登录失败 ({status})",
      context: "上下文",
      session: "会话",
      slash_ml: "切换多行输入",
      slash_full: "切换到全屏显示模式",
      slash_tour: "打开入门教程",
      slash_onboarding: "打开入门教程",
      slash_terminal_setup: "检查终端、剪贴板、输入和沙箱等问题",
      slash_terminal_check: "检查终端、剪贴板、输入和沙箱等问题",
      slash_terminal_info: "检查终端、剪贴板、输入和沙箱等问题",
      slash_hooks_list: "列出已加载的 Hooks",
      slash_hooks_trust: "信任指定 Hook",
      slash_hooks_add: "添加自定义 Hook",
      slash_hooks_remove: "移除自定义 Hook",
      slash_hooks_untrust: "取消信任指定 Hook",
      slash_find: "在当前会话回看中搜索",
      slash_jump: "跳转到指定轮次或位置",
      slash_timeline: "打开会话时间线",
      slash_expand: "展开当前选中的内容块",
      slash_docx: "创建、读取和编辑 Word 文档（.docx）",
      slash_pdf: "创建、合并、拆分 PDF，并从中提取内容",
      slash_pptx: "创建、读取和编辑 PowerPoint 演示文稿（.pptx）",
      slash_create_skill: "通过对话创建自定义技能",
      slash_create_workflow: "编写新的多 Agent 工作流",
      slash_build_with_ai: "在 SpaceXAI 上构建 AI 应用",
      slash_code_review: "严格审查可维护性、抽象质量和过大文件",
      slash_design: "撰写设计文档并循环评审，直到达成共识",
      slash_execute_plan: "按设计文档里的 PR 计划逐步实现",
      slash_implement: "实现、审查、修复循环，直到没有问题",
      slash_review: "评审本地改动、分支或 GitHub PR",
      slash_pr_babysit: "盯 PR、修 CI、处理评审意见和冲突",
      slash_game_asset_core: "游戏素材的引擎就绪默认规则",
      slash_game_animation_frames: "面向视频、能循环播放的动画帧",
      slash_game_character_consistency: "同一角色在每张图里都保持一致",
      slash_game_tilesets: "无缝拼接的地形图块和过渡",
      slash_game_ui_icons: "游戏 UI 套件和图标集",
      slash_resume_claude: "从最近的 Claude Code 会话继续",
      slash_resume_codex: "从最近的 Codex 会话继续",
      slash_resume_cursor: "从最近的 Cursor 会话继续",
      slash_skill_design_principles: "编写和修改技能时用的简洁原则",
      slash_neon: "Neon 平台总览：Postgres、Auth、Data API，以及对象存储、函数和 AI Gateway",
      slash_neon_postgres: "Neon Serverless Postgres 的使用指南与最佳实践",
      slash_neon_postgres_branches: "为测试和开发选择并创建合适的 Neon 分支",
    },
    en: {
      newSession: "New Chat",
      themeLight: "Light",
      themeDark: "Dark",
      collapseSidebar: "Collapse sidebar",
      expandSidebar: "Expand sidebar",
      searchSessions: "Search chats",
      search: "Search",
      finderOps: "Actions",
      finderNew: "New Chat",
      finderToday: "Today",
      finderYesterday: "Yesterday",
      finderWeek: "Last 7 days",
      finderOlder: "Older",
      finderPreview: "Select a conversation to preview",
      finderGo: "Go",
      finderCurrent: "Current",
      finderResize: "Resize",
      finderHidePreview: "Hide conversation preview",
      finderShowPreview: "Show conversation preview",
      finderEdit: "Edit",
      finderDelete: "Delete",
      rename: "Rename",
      delete: "Delete",
      pin: "Pin",
      unpin: "Unpin",
      pinnedGroup: "Pinned",
      sessionMenu: "Chat actions",
      renamePlaceholder: "Chat title",
      deleteChatTitle: "Delete this chat?",
      deleteChatBody: "This cannot be undone.",
      confirmDelete: "Delete",
      contextUsage: "Context usage",
      sessionList: "Chat list",
      status: "Status",
      workspace: "Workspace",
      wsUp: "Parent",
      wsRefresh: "Refresh",
      wsPack: "Zip working directory",
      wsPacking: "Zipping",
      wsDownload: "Download",
      wsAtRef: "Add to chat",
      wsDeleteTitle: "Delete “{name}”?",
      wsDeleteBody: "This cannot be undone.",
      wsEmpty: "This folder is empty",
      wsTruncated: "Too many entries; showing the first 2000",
      wsSkipHint: "Zip skips .git, node_modules, target, and __pycache__",
      copyTurn: "Copy this turn",
      downloadMd: "Download markdown",
      sessionInfo: "Session info",
      logout: "Log out",
      promptPlaceholder: "Type / to use slash commands",
      promptPh0: "Type / to use slash commands",
      promptPh1: "Type / to use slash commands",
      promptPh2: "Type @ to mention workspace files",
      promptPh3: "Drop files into the chat",
      attach: "Attach",
      processing: "Processing",
      pickCwd: "Choose working directory",
      model: "Model",
      send: "Send",
      stop: "Stop",
      process: "Process",
      close: "Close",
      sessionUsage: "Session usage",
      sinceStart: "since start or last resume",
      host: "Host",
      langZh: "Chinese",
      langEn: "English",
      enter: "Enter",
      loginTitle: "Sign in · GGOK",
      sessionBusy: "Session is occupied by the terminal",
      occupyForeign: "Occupied by terminal grok",
      occupyObserve: "grok is still running; control plane is not connected",
      justNow: "just now",
      minutesAgo: "{n}m ago",
      hoursAgo: "{n}h ago",
      daysAgo: "{n}d ago",
      copied: "Copied",
      copyFailed: "Copy failed",
      noSessions: "No chats",
      copy: "Copy",
      copyCode: "Copy",
      edit: "Edit",
      working: "Working",
      workedSeconds: "Worked {n}s",
      itemCount: "{n} items",
      itemCountOne: "1 item",
      workDone: "Done",
      userStopped: "Stopped by user",
      effortLow: "Low",
      effortMedium: "Medium",
      effortHigh: "High",
      effortXhigh: "Extra High",
      effortLowDesc: "Faster, uses less",
      effortMediumDesc: "Balanced speed and quality",
      effortHighDesc: "More reasoning",
      effortXhighDesc: "Maximum reasoning",
      slashTuiOnly: "This command is for the CLI terminal. WebUI has no equivalent.",
      noModels: "No models yet",
      allowTool: "Allow this tool?",
      allow: "Allow",
      deny: "Deny",
      emptySession: "This chat has no messages yet.",
      loading: "Loading…",
      loadFailed: "Failed to load: {e}",
      agentError: "error",
      requestFailed: "Request failed",
      remaining: "remaining",
      left: "left",
      noModelCalls: "no model calls yet in this session.",
      inputTokens: "Input tokens",
      outputTokens: "Output tokens",
      totalTokens: "Total tokens",
      modelCalls: "Model calls",
      apiTime: "API time",
      cost: "Cost",
      cachedNote: "({n} cached)",
      reasoningNote: "({n} reasoning)",
      inOut: "{inn} in / {out} out",
      weeklyLimit: "Weekly limit",
      monthlyLimit: "Monthly limit",
      dailyLimit: "Daily limit",
      usageLimit: "Usage limit",
      resetsSoon: "resets soon",
      resetsInDh: "resets in {d}d {h}h",
      resetsInHm: "resets in {h}h {m}m",
      resetsInM: "resets in {m}m",
      loadingUsage: "Loading usage…",
      couldntLoadUsage: "Couldn't load usage",
      plan: "Plan",
      account: "Account",
      usedPct: "{n}% used",
      leftPct: "{n}% left",
      used: "Used",
      remaining: "Left",
      resets: "Resets",
      user: "User",
      hostname: "Hostname",
      lanIpv4: "LAN IPv4",
      wanIpv4: "WAN IPv4",
      cpu: "CPU",
      memory: "Memory",
      disk: "Disk {path}",
      used: "Used",
      window: "Window",
      remainingLabel: "Remaining",
      input: "Input",
      output: "Output",
      total: "Total",
      cachedLabel: "Cached",
      id: "ID",
      cwd: "CWD",
      title: "Title",
      emptyQueue: "(empty)",
      sendNow: "Send now",
      recallQueue: "Recall",
      allowedRoots: "Start here",
      atNoMatch: "No matching files",
      chooseThisDir: "Choose this folder",
      parentDir: "← Parent",
      mcps: "MCP",
      mcpSearch: "Search servers",
      mcpRefresh: "Refresh",
      mcpEmpty: "No MCP servers yet",
      mcpLocal: "Local",
      mcpDisabled: "Disabled",
      mcpEnable: "Enable",
      mcpDisable: "Disable",
      mcpRemove: "Remove",
      mcpConfirmRemove: "Confirm remove",
      mcpAdd: "Add",
      mcpName: "Name",
      mcpCommand: "Command or URL",
      mcpArgs: "Args, space-separated, optional",
      mcpScopeUser: "User",
      mcpScopeProject: "Project",
      mcpAddNeed: "Name and command are required",
      mcpTools: "Tools",
      mcpNoTools: "No tools yet",
      mcpHealth: "Status",
      skillFiles: "Files",
      skillLoading: "Loading skill…",
      skillBinary: "Binary file",
      plugins: "Plugins",
      extNav: "Plugins · MCP · Skills",
      marketplace: "Marketplace",
      connectors: "Connectors",
      skills: "Skills",
      newConnector: "New Connector",
      newSkill: "New Skill",
      writeSkill: "Write skill manually",
      uploadSkill: "Upload skill file",
      createSkillAi: "Create skill in chat",
      personal: "Personal",
      quickSkills: "Quick Skills",
      quickEmpty: "No matching quick skills",
      slash_new: "Start a fresh session and clear the current conversation",
      slash_resume: "Open the session picker to reload a previous session from disk",
      slash_dashboard: "Open the Agent Dashboard: live roster of top-level sessions",
      slash_compact: "Compress conversation history to reclaim context-window space",
      slash_context: "Show how the context window is being used",
      slash_session_info: "Show session details: auth method, model, turn count, and context usage",
      slash_fork: "Branch the current session into a new agent, keeping history up to this point",
      slash_rewind: "Roll the conversation back to an earlier turn and discard everything after it",
      slash_undo: "Roll the conversation back to an earlier turn and discard everything after it",
      slash_copy: "Copy the most recent response's source markdown to the clipboard",
      slash_export: "Export the conversation to a file or the clipboard",
      slash_quit: "Quit the application",
      slash_exit: "Quit the application",
      slash_home: "Leave the current session and return to the welcome screen",
      slash_welcome: "Leave the current session and return to the welcome screen",
      slash_delete: "Delete the current session's history",
      slash_rename: "Rename the current session",
      slash_title: "Rename the current session",
      slash_model: "Switch models. Accepts a model ID or display name (case-insensitive), and for reasoning models you can add an effort level as a second argument",
      slash_m: "Switch models. Accepts a model ID or display name (case-insensitive), and for reasoning models you can add an effort level as a second argument",
      slash_effort: "Set reasoning effort on the current model without reselecting it",
      slash_always_approve: "Toggle always-approve permission mode",
      slash_auto: "Toggle auto permission mode",
      slash_multiline: "Toggle multiline input",
      slash_history: "Open prompt-history search for this session",
      slash_compact_mode: "Toggle compact display — less padding and tighter spacing for denser output",
      slash_vim_mode: "Toggle vim-style scrollback keys (j/k, h/l, g/G, y/Y, and so on)",
      slash_edit_prompt: "Open an external editor for the prompt",
      slash_minimal: "Switch the current session to minimal render mode",
      slash_fullscreen: "Switch the current session to fullscreen render mode",
      slash_plan: "Enter plan mode",
      slash_view_plan: "Open a preview of the current saved plan",
      slash_show_plan: "Open a preview of the current saved plan",
      slash_plan_view: "Open a preview of the current saved plan",
      slash_memory: "Browse, view, and manage saved memories",
      slash_mem: "Browse, view, and manage saved memories",
      slash_flush: "Save the current session's knowledge to memory right now",
      slash_dream: "Run memory consolidation — merge session logs into organized topics",
      slash_remember: "Save a note to memory immediately",
      slash_hooks: "Open the extensions modal on the Hooks tab",
      slash_plugins: "Open the extensions modal on the Plugins tab",
      slash_marketplace: "Open the extensions modal on the Marketplace tab",
      slash_skills: "Open the extensions modal on the Skills tab",
      slash_imagine: "Generate an image from a text description",
      slash_imagine_video: "Generate a video from a text or image description",
      slash_loop: "Run a prompt on a recurring interval",
      slash_goal: "Set, manage, or check an autonomous goal",
      slash_deep_research: "Kick off a background research workflow",
      slash_workflow: "Launch a saved workflow, or manage a running one",
      slash_workflows: "Open the extensions modal on the Workflows tab",
      slash_theme: "Switch the color theme",
      slash_t: "Switch the color theme",
      slash_feedback: "Report an issue or send feedback",
      slash_btw: "Send an aside to the agent without interrupting the current task",
      slash_mcps: "Open the MCP servers management modal",
      slash_doctor: "Check the session for terminal, clipboard, color, input, notification, and sandbox issues",
      slash_release_notes: "View release notes for the current version",
      slash_changelog: "View release notes for the current version",
      slash_docs: "Browse the built-in How-to Guides or open the online docs",
      slash_howto: "Browse the built-in How-to Guides or open the online docs",
      slash_guides: "Browse the built-in How-to Guides or open the online docs",
      slash_tutorial: "Open the onboarding tutorial",
      slash_import_claude: "Open the Claude import modal to bring over ~/.claude settings",
      slash_config_agents: "Open the agents modal to view and manage agent definitions",
      slash_agents: "Open the agents modal to view and manage agent definitions",
      slash_personas: "Create, edit, and delete personas",
      slash_login: "Log in or re-authenticate without leaving the session",
      slash_logout: "Log out and return to the login screen",
      slash_usage: "View credit usage or manage billing",
      slash_cost: "View credit usage or manage billing",
      slash_privacy: "Open settings for coding data, retention, and training",
      slash_settings: "Open the settings modal",
      slash_config: "Open the settings modal",
      slash_preferences: "Open the settings modal",
      slash_prefs: "Open the settings modal",
      slash_timestamps: "Toggle message timestamps on or off",
      slash_clear: "Start a fresh session and clear the current conversation",
      slash_agents_dashboard: "Open the Agent Dashboard: live roster of top-level sessions",
      slash_sessions: "Open the Agent Dashboard: live roster of top-level sessions",
      slash_status: "Show session details: auth method, model, turn count, and context usage",
      slash_info: "Show session details: auth method, model, turn count, and context usage",
      skillQ_docx: "Word Documents",
      skillQ_docxDesc: "Create, read, and edit Word documents (.docx)",
      skillQ_pdf: "PDFs",
      skillQ_pdfDesc: "Create, merge, split, and extract from PDFs",
      skillQ_pptx: "Presentations",
      skillQ_pptxDesc: "Create, read, and edit PowerPoint presentations (.pptx)",
      skillQ_create_skill: "Skill Creator",
      skillQ_create_skillDesc: "Build new custom skills through conversation",
      skillQ_create_workflow: "Create Workflow",
      skillQ_create_workflowDesc: "Author a new multi-agent workflow",
      skillQ_build_with_ai: "Build with AI",
      skillQ_build_with_aiDesc: "Build AI apps on SpaceXAI",
      skillQ_code_review: "Code Review",
      skillQ_code_reviewDesc: "Strict maintainability review for abstraction quality and giant files",
      skillQ_design: "Design",
      skillQ_designDesc: "Write a design doc and review it until consensus",
      skillQ_execute_plan: "Execute Plan",
      skillQ_execute_planDesc: "Implement the PR plan from a design document",
      skillQ_implement: "Implement",
      skillQ_implementDesc: "Implement, review, and fix until there are no issues",
      skillQ_review: "Review",
      skillQ_reviewDesc: "Review local changes, a branch, or a GitHub PR",
      skillQ_pr_babysit: "PR Babysit",
      skillQ_pr_babysitDesc: "Watch PRs, fix CI, handle review comments and conflicts",
      skillQ_imagine: "Imagine",
      skillQ_imagineDesc: "Prompting and workflow for Imagine image tools",
      skillQ_game_asset_core: "Game Asset Core",
      skillQ_game_asset_coreDesc: "Engine-ready defaults for game assets",
      skillQ_game_animation_frames: "Game Animation Frames",
      skillQ_game_animation_framesDesc: "Video-first animation frames that actually cycle",
      skillQ_game_character_consistency: "Character Consistency",
      skillQ_game_character_consistencyDesc: "Keep the same character consistent in every image",
      skillQ_game_tilesets: "Game Tilesets",
      skillQ_game_tilesetsDesc: "Seamless tiles and transition sets that actually tile",
      skillQ_game_ui_icons: "Game UI Icons",
      skillQ_game_ui_iconsDesc: "Game UI kits and icon sets",
      skillQ_resume_claude: "Resume Claude",
      skillQ_resume_claudeDesc: "Continue from a recent Claude Code session",
      skillQ_resume_codex: "Resume Codex",
      skillQ_resume_codexDesc: "Continue from a recent Codex session",
      skillQ_resume_cursor: "Resume Cursor",
      skillQ_resume_cursorDesc: "Continue from a recent Cursor session",
      skillQ_skill_design_principles: "Skill Design Principles",
      skillQ_skill_design_principlesDesc: "Concise principles for writing and editing skills",
      skillDocuments: "Documents",
      skillGame: "Game",
      skillResume: "Resume",
      skillBuiltin: "Built-in",
      skillName: "Name",
      skillDesc: "Description",
      skillBody: "Body",
      skillCreateNeed: "Name and description are required",
      extAdd: "Add",
      extAdded: "Added",
      extSearch: "Search...",
      skillEmpty: "No skills yet",
      extBack: "Back",
      extAllSkills: "All skills",
      extAllMarket: "All marketplace",
      extAllPlugins: "All plugins",
      extAllConnectors: "All connectors",
      extTry: "Try it",
      extTryNow: "Try now",
      cancel: "Cancel",
      extTrustTitle: "Trust and install?",
      extTrustBody: "This will install {name}. You need to trust this plugin first.",
      pluginSearch: "Search plugins",
      pluginEmpty: "No plugins installed",
      marketEmpty: "No matching marketplace plugins",
      pluginInstall: "Install",
      pluginInstalling: "Installing…",
      pluginInstallOk: "Installed {name}",
      pluginConfirmInstall: "Trust and install",
      pluginUninstall: "Uninstall",
      pluginConfirmUninstall: "Confirm uninstall",
      pluginUpdate: "Update",
      pluginInstalled: "Installed",
      pluginSkills: "{n} skills",
      pluginSourcePh: "git URL / GitHub shorthand / local path",
      pluginInstallPh: "Name, git URL, or GitHub shorthand",
      pluginAddSource: "Add source",
      pluginRemoveSource: "Remove source",
      pluginConfirmRemoveSource: "Remove source and its plugins",
      pluginAddNeed: "Enter a source or plugin name",
      pickCwdFirst: "Choose a working directory first",
      runningTool: "Running {name}{preview}",
      ranTools: "Ran {n} {name}",
      rateLimited: "Too many failed attempts, try again in 15 minutes",
      badToken: "Incorrect token",
      loginFailed: "Sign-in failed ({status})",
      context: "Context",
      slash_ml: "Toggle multiline input",
      slash_full: "Switch the current session to fullscreen render mode",
      slash_tour: "Open the onboarding tutorial",
      slash_onboarding: "Open the onboarding tutorial",
      slash_terminal_setup: "Check the session for terminal, clipboard, color, input, notification, and sandbox issues",
      slash_terminal_check: "Check the session for terminal, clipboard, color, input, notification, and sandbox issues",
      slash_terminal_info: "Check the session for terminal, clipboard, color, input, notification, and sandbox issues",
      slash_hooks_list: "List loaded hooks",
      slash_hooks_trust: "Trust a hook",
      slash_hooks_add: "Add a custom hook",
      slash_hooks_remove: "Remove a custom hook",
      slash_hooks_untrust: "Revoke trust for a hook",
      slash_find: "Search the current session scrollback",
      slash_jump: "Jump to a specific turn or position",
      slash_timeline: "Open the conversation timeline",
      slash_expand: "Expand the selected block",
      slash_docx: "Create, read, and edit Word documents (.docx)",
      slash_pdf: "Create, merge, split, and extract from PDFs",
      slash_pptx: "Create, read, and edit PowerPoint presentations (.pptx)",
      slash_create_skill: "Build new custom skills through conversation",
      slash_create_workflow: "Author a new multi-agent workflow",
      slash_build_with_ai: "Build AI apps on SpaceXAI",
      slash_code_review: "Strict maintainability review for abstraction quality and giant files",
      slash_design: "Write a design doc and review it until consensus",
      slash_execute_plan: "Implement the PR plan from a design document",
      slash_implement: "Implement, review, and fix until there are no issues",
      slash_review: "Review local changes, a branch, or a GitHub PR",
      slash_pr_babysit: "Watch PRs, fix CI, handle review comments and conflicts",
      slash_game_asset_core: "Engine-ready defaults for game assets",
      slash_game_animation_frames: "Video-first animation frames that actually cycle",
      slash_game_character_consistency: "Keep the same character consistent in every image",
      slash_game_tilesets: "Seamless tiles and transition sets that actually tile",
      slash_game_ui_icons: "Game UI kits and icon sets",
      slash_resume_claude: "Continue from a recent Claude Code session",
      slash_resume_codex: "Continue from a recent Codex session",
      slash_resume_cursor: "Continue from a recent Cursor session",
      slash_skill_design_principles: "Concise principles for writing and editing skills",
      slash_neon: "Overview of the Neon platform for apps and agents, spanning Postgres, Auth, Data API, and related services",
      slash_neon_postgres: "Guides and best practices for working with Neon Serverless Postgres",
      slash_neon_postgres_branches: "Choose and create the right Neon branch type for testing and development",
      session: "Session"
    }
  };

  function detect() {
    try {
      const stored = localStorage.getItem(KEY);
      if (stored === "zh" || stored === "en") return stored;
    } catch (err) {
    }
    const list = (typeof navigator !== "undefined" && navigator.languages) || [];
    const nav = String(
      list[0] ||
        (typeof navigator !== "undefined" && (navigator.language || navigator.userLanguage)) ||
        ""
    ).toLowerCase();
    return nav.indexOf("zh") === 0 ? "zh" : "en";
  }

  function htmlLang(lang) {
    return lang === "zh" ? "zh-CN" : "en";
  }

  function t(key, vars) {
    const pack = dict[I18n.lang] || dict.en;
    let s = pack[key] || dict.en[key] || key;
    if (vars) {
      s = String(s).replace(/\{(\w+)\}/g, (_, k) => (vars[k] == null ? "" : String(vars[k])));
    }
    return s;
  }

  const TIP_DELAY = 700;
  const TIP_SKIP = 300;
  let tipEl = null;
  let tipTimer = 0;
  let tipLeaveTimer = 0;
  let tipTarget = null;
  let tipSkipUntil = 0;
  let tipBound = false;

  function ensureTipEl() {
    if (tipEl && tipEl.isConnected) return tipEl;
    tipEl = document.getElementById("hover-tip");
    if (!tipEl && document.body) {
      tipEl = document.createElement("div");
      tipEl.id = "hover-tip";
      tipEl.hidden = true;
      tipEl.setAttribute("role", "tooltip");
      document.body.appendChild(tipEl);
    }
    return tipEl;
  }

  function isTruncated(el) {
    return !!(el && el.scrollWidth > el.clientWidth + 1);
  }

  function setTip(el, text) {
    if (!el) return;
    const s = String(text == null ? "" : text);
    if (s) el.setAttribute("data-tip", s);
    else el.removeAttribute("data-tip");
    el.removeAttribute("title");
    if (tipTarget === el) {
      const next = tipTextFor(el);
      if (!next) hideTip(false);
      else if (tipEl && !tipEl.hidden) {
        tipEl.textContent = next;
        placeTip(el);
      }
    }
  }

  function hideTip(armSkip) {
    const wasOpen = tipEl && !tipEl.hidden;
    clearTimeout(tipTimer);
    tipTimer = 0;
    tipTarget = null;
    if (tipEl && wasOpen) {
      clearTimeout(tipLeaveTimer);
      tipEl.classList.add("is-leaving");
      const node = tipEl;
      tipLeaveTimer = setTimeout(() => {
        if (!node.classList.contains("is-leaving")) return;
        node.hidden = true;
        node.classList.remove("is-leaving");
        node.textContent = "";
        node.removeAttribute("data-side");
        node.style.animation = "";
      }, 50);
    } else if (tipEl) {
      clearTimeout(tipLeaveTimer);
      tipEl.hidden = true;
      tipEl.classList.remove("is-leaving");
      tipEl.textContent = "";
      tipEl.removeAttribute("data-side");
    }
    if (armSkip && wasOpen) tipSkipUntil = Date.now() + TIP_SKIP;
  }

  function preferredSide(el) {
    const s = el.getAttribute("data-tip-side");
    if (s === "top" || s === "bottom" || s === "left" || s === "right") return s;
    if (el.classList.contains("sess") || el.classList.contains("proj-toggle")) return "right";
    return "bottom";
  }

  function sideOrder(pref) {
    const opp = { top: "bottom", bottom: "top", left: "right", right: "left" };
    const rest = ["bottom", "top", "right", "left"].filter((s) => s !== pref && s !== opp[pref]);
    return [pref, opp[pref]].concat(rest);
  }

  function posForSide(side, r, tw, th, gap) {
    if (side === "right") return { left: r.right + gap, top: r.top + (r.height - th) / 2 };
    if (side === "left") return { left: r.left - tw - gap, top: r.top + (r.height - th) / 2 };
    if (side === "top") return { left: r.left + (r.width - tw) / 2, top: r.top - th - gap };
    return { left: r.left + (r.width - tw) / 2, top: r.bottom + gap };
  }

  function placeTip(anchor) {
    const el = ensureTipEl();
    if (!el || !anchor) return;
    const r = anchor.getBoundingClientRect();
    const gap = 6;
    const pad = 8;
    el.style.left = "0px";
    el.style.top = "0px";
    const tw = el.offsetWidth;
    const th = el.offsetHeight;
    const vw = window.innerWidth;
    const vh = window.innerHeight;
    let chosen = preferredSide(anchor);
    let left = 0;
    let top = 0;
    let found = false;
    for (const side of sideOrder(chosen)) {
      const pos = posForSide(side, r, tw, th, gap);
      if (
        pos.left >= pad &&
        pos.top >= pad &&
        pos.left + tw <= vw - pad &&
        pos.top + th <= vh - pad
      ) {
        chosen = side;
        left = pos.left;
        top = pos.top;
        found = true;
        break;
      }
    }
    if (!found) {
      const pos = posForSide(chosen, r, tw, th, gap);
      left = pos.left;
      top = pos.top;
    }
    if (left < pad) left = pad;
    if (top < pad) top = pad;
    if (left + tw > vw - pad) left = Math.max(pad, vw - pad - tw);
    if (top + th > vh - pad) top = Math.max(pad, vh - pad - th);
    el.dataset.side = chosen;
    el.style.left = left + "px";
    el.style.top = top + "px";
  }

  function showTip(anchor, text) {
    const el = ensureTipEl();
    if (!el || !anchor || !text) return;
    clearTimeout(tipLeaveTimer);
    el.classList.remove("is-leaving");
    el.textContent = text;
    el.hidden = false;
    el.style.animation = "none";
    placeTip(anchor);
    el.offsetHeight;
    el.style.animation = "";
    tipTarget = anchor;
  }

  function tipTextFor(el) {
    if (!el || !el.getAttribute) return "";
    if (el.getAttribute("aria-expanded") === "true") return "";
    const full = el.getAttribute("data-tip") || "";
    if (!full) return "";
    if (el.classList.contains("sess")) {
      const name = el.querySelector(".name");
      if (!name) return full;
      return isTruncated(name) ? full : "";
    }
    if (el.classList.contains("proj-toggle")) {
      return full !== (el.textContent || "").trim() ? full : "";
    }
    if (el.hasAttribute("data-tip-overflow")) {
      const target = el.querySelector("[data-tip-overflow-target]") || el;
      return isTruncated(target) ? full : "";
    }
    return full;
  }

  function scheduleTip(anchor) {
    const text = tipTextFor(anchor);
    if (!anchor || !text) {
      hideTip(false);
      return;
    }
    if (tipTarget === anchor) return;
    tipTarget = anchor;
    clearTimeout(tipTimer);
    const open = tipEl && !tipEl.hidden;
    const delay = open || Date.now() < tipSkipUntil ? 0 : TIP_DELAY;
    tipTimer = setTimeout(() => {
      showTip(anchor, text);
    }, delay);
  }

  function tipAnchorFrom(node) {
    if (!node || !node.closest) return null;
    return node.closest("[data-tip]");
  }

  function bindTipEngine() {
    if (tipBound) return;
    tipBound = true;
    document.addEventListener("pointerover", (e) => {
      if (e.pointerType && e.pointerType !== "mouse") return;
      const item = tipAnchorFrom(e.target);
      if (!item) return;
      scheduleTip(item);
    });
    document.addEventListener("pointerout", (e) => {
      const item = tipAnchorFrom(e.target);
      if (!item) return;
      const rel = e.relatedTarget && e.relatedTarget.nodeType === 1 ? e.relatedTarget : null;
      if (rel && item.contains(rel)) return;
      const next = tipAnchorFrom(rel);
      if (next) return;
      hideTip(true);
    });
    document.addEventListener("pointerdown", () => hideTip(false), true);
    document.addEventListener("scroll", () => hideTip(true), true);
    window.addEventListener("resize", () => hideTip(false));
    document.addEventListener("keydown", (e) => {
      if (e.key === "Escape") hideTip(false);
    });
  }

  function apply(root) {
    const el = root || document;
    if (!el.querySelectorAll) return;
    el.querySelectorAll("[data-i18n]").forEach((n) => {
      n.textContent = t(n.getAttribute("data-i18n"));
    });
    el.querySelectorAll("[data-i18n-title]").forEach((n) => {
      const s = t(n.getAttribute("data-i18n-title"));
      setTip(n, s);
      if (!n.hasAttribute("aria-label") && !n.hasAttribute("data-i18n-aria")) {
        n.setAttribute("aria-label", s);
      }
    });
    el.querySelectorAll("[data-i18n-placeholder]").forEach((n) => {
      n.setAttribute("placeholder", t(n.getAttribute("data-i18n-placeholder")));
    });
    el.querySelectorAll("[data-i18n-aria]").forEach((n) => {
      n.setAttribute("aria-label", t(n.getAttribute("data-i18n-aria")));
    });
    const docKey = document.documentElement.getAttribute("data-i18n-doc-title");
    if (docKey) document.title = t(docKey);
  }

  function syncToggle(btn) {
    if (!btn) return;
    const label = t(I18n.lang === "zh" ? "langZh" : "langEn");
    setTip(btn, label);
    btn.setAttribute("aria-label", label);
  }

  function ready() {
    document.documentElement.removeAttribute("data-i18n-pending");
    document.documentElement.dataset.i18nReady = "1";
  }

  function setLang(lang, persist) {
    const next = lang === "zh" ? "zh" : "en";
    I18n.lang = next;
    document.documentElement.lang = htmlLang(next);
    document.documentElement.dataset.lang = next;
    if (persist) {
      try {
        localStorage.setItem(KEY, next);
      } catch (err) {
      }
    }
    apply(document);
    document.querySelectorAll("#lang-btn").forEach(syncToggle);
    ready();
    document.dispatchEvent(new CustomEvent("i18n-change", { detail: { lang: next } }));
  }

  const I18n = {
    KEY,
    lang: detect(),
    t,
    apply,
    setLang,
    setTip,
    hideTip,
    detect,
    locale() {
      return I18n.lang === "zh" ? "zh-CN" : "en-US";
    },
    bindToggle(btn) {
      if (!btn || btn.dataset.i18nBound) return;
      btn.dataset.i18nBound = "1";
      syncToggle(btn);
      btn.addEventListener("click", () => {
        setLang(I18n.lang === "zh" ? "en" : "zh", true);
      });
    }
  };

  window.I18n = I18n;
  document.documentElement.lang = htmlLang(I18n.lang);
  document.documentElement.dataset.lang = I18n.lang;

  const boot = () => {
    ensureTipEl();
    bindTipEngine();
    apply(document);
    document.querySelectorAll("#lang-btn").forEach((b) => I18n.bindToggle(b));
    ready();
  };
  bindTipEngine();
  if (document.readyState === "loading") document.addEventListener("DOMContentLoaded", boot);
  else boot();
})();
