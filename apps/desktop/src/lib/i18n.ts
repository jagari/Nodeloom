// The dashboard's words in English and Korean. No library: two dictionaries
// with the same keys, `{name}` placeholders and per-language plural forms.
// Pure (no Svelte), so the desktop shell's pages can use it too; the current
// language as reactive state lives in locale.svelte.ts.

export type Lang = 'en' | 'ko';
export const LANGS: readonly Lang[] = ['en', 'ko'];

/** A fixed text, or forms picked by the `count` parameter (`Intl.PluralRules`). */
export type Message = string | { one: string; other: string };
export type Params = Record<string, string | number>;

const en = {
  // Header and connection
  'conn.connecting': 'Connecting…',
  'conn.live': 'Live',
  'conn.offline': 'Master unreachable, retrying…',
  'conn.unauthorized': 'Wrong admin token',
  'conn.disconnect': 'Disconnect',
  'kpi.label': 'Cluster summary',
  'kpi.online': 'Machines online',
  'kpi.running': 'Pieces running',
  'kpi.cpu': 'Average CPU',

  'connect.title': 'Connect to a master',
  'connect.intro':
    'Start {master} on one machine, then enter its address and the admin token it prints the first time (also saved as {file} in its data folder).',
  'connect.address': 'Master address',
  'connect.badAddress': 'That does not look like an address.',
  'connect.token': 'Admin token',
  'connect.submit': 'Connect',

  // Counted things. Korean counts with a counter word and no plural.
  'count.machine': { one: '{count} machine', other: '{count} machines' },
  'count.piece': { one: '{count} piece', other: '{count} pieces' },
  'count.frame': { one: '{count} frame', other: '{count} frames' },
  'count.question': { one: '{count} question', other: '{count} questions' },
  'count.part': { one: '{count} part', other: '{count} parts' },
  'count.group': { one: '{count} group', other: '{count} groups' },
  'count.summary': { one: '{count} summary', other: '{count} summaries' },
  'count.page': { one: '{count} page', other: '{count} pages' },
  'count.char': { one: '{count} character', other: '{count} characters' },
  'count.step': { one: '{count} step', other: '{count} steps' },
  'count.core': { one: '{count} core', other: '{count} cores' },
  // A job's unit before a range, as in "frames 201–400".
  'unit.frame': { one: 'frame', other: 'frames' },
  'unit.question': { one: 'question', other: 'questions' },
  'unit.part': { one: 'part', other: 'parts' },
  'unit.group': { one: 'group', other: 'groups' },
  'unit.summary': { one: 'summary', other: 'summaries' },

  'common.cancel': 'Cancel',
  'common.copy': 'Copy',
  'common.copied': 'Copied',
  'common.starting': 'Starting…',
  'common.working': 'Working…',

  // Cluster map and machine list
  'map.title': 'Cluster',
  'map.master': 'Master',
  'map.summary': {
    one: 'Master with {online} of {count} machine connected, {working} working.',
    other: 'Master with {online} of {count} machines connected, {working} working.',
  },
  'map.none': 'No machine paired yet',
  'node.offline': 'offline',
  'node.idle': 'Idle',
  'node.working': 'Working',
  'node.progress': '{range} on {name}',
  'node.waitingReport': 'Waiting for the first report…',
  'node.memoryOf': '{used} of {total}',
  'mem.high': 'High',
  'mem.critical': 'Critical',
  'machines.title': 'Machines',
  'machines.online': '({count} online)',
  'machines.none': 'No machine is online yet. See {add}.',
  'col.machine': 'Machine',
  'col.status': 'Status',
  'col.work': 'Working on',
  'col.cpu': 'CPU',
  'col.memory': 'Memory',

  // Approval and paired machines
  'pending.title': 'Waiting for approval',
  'pending.hint': 'Approve a machine only if it shows the same code. On the machine, the agent prints it at start, or run {command}.',
  'pending.codeLabel': 'Confirmation code {code}',
  'pending.confirm': 'Approve {name} with code {code}? Check the code on the machine itself first.',
  'pending.confirmFrom': 'Approve {name} with code {code} from {address}? Check the code on the machine itself first.',
  'pending.approveAnyway': 'Approve anyway',
  'pending.approve': 'Approve',
  'pending.ignore': 'Ignore',
  'flag.sameCode': 'Another waiting machine shows the same code.',
  'flag.sameId': 'Another waiting machine uses the same id.',
  'flag.sameName': 'Another waiting machine has the same name.',
  'flag.pairedName': 'A paired machine already has this name.',
  'paired.title': 'Paired machines ({count})',
  'paired.online': 'Online',
  'paired.offline': 'Offline',
  'paired.remove': 'Remove',
  'paired.removing': 'Removing…',
  'paired.removeForGood': 'Remove for good',
  'paired.explain':
    'Removing {name} disconnects it at once and refuses its key from then on. To add it again, run {reset} on it and approve it here. If it was started with {token}, that machine also holds the admin token: rotate it with {rotate}.',

  // Add a machine. "Join a cluster" and "Enter the address" are buttons in
  // the desktop app's own page, so they stay as that page shows them.
  'add.title': 'Add a machine',
  'add.step1': 'On the other computer, install Nodeloom (the same installer as this one) and open it.',
  'add.step2': 'Choose {join}. On this network it finds this cluster by itself.',
  'add.step3': 'It appears here under {waiting} with a 6-digit code. Approve it if the code is the same as on that computer.',
  'add.trouble': 'Not showing up?',
  'add.network': 'Both computers must be on the same network (not a guest Wi-Fi or a VPN).',
  'add.firewall':
    'On Windows, allow Nodeloom through the firewall when asked, on this computer and the other one. If the network is set to {pub}, switch it to {priv}.',
  'add.public': 'Public',
  'add.private': 'Private',
  'add.manual': 'On the other computer, choose {enter} and type:',
  'add.cli': 'On a Raspberry Pi, Jetson or server (command line):',

  // Start a job
  'start.title': 'Start a job',
  'start.noMachine': 'Add a machine first.',
  'start.noFake': 'No machine has the test runner. Start agents with {flag}.',
  'start.demoHint': 'Demo job: 600 test frames split across {machines}, about {seconds} s. A test that only waits; nothing is rendered.',
  'start.demo': 'Demo job',
  'ai.title': 'Ask the local AI',
  'ai.none': 'No machine runs a local LLM yet. Install {ollama} and a model on the machines (for example {command}), then restart Nodeloom there.',
  'ai.hint': '{questions} split across {machines}; each answers with its own model.',
  'ai.model': 'Model',
  'ai.questions': 'Questions ({count})',
  'ai.perLine': 'One question per line',
  'ai.ask': 'Ask the AI',
  'prompt.none': 'Write at least one question, one per line.',
  'prompt.tooMany': 'At most {max} questions.',
  'prompt.tooLong': 'Question {n} is longer than {max} characters.',
  'custom.title': 'Custom job',
  'custom.runner': 'Runner',
  'custom.blender': 'Blender render',
  'custom.fake': 'Test (sleep only)',
  'custom.addFirst': 'Add a machine first',
  'custom.blendFile': '.blend file (relative to each machine’s work folder)',
  'custom.millis': 'Milliseconds per frame',
  'custom.first': 'First frame',
  'custom.last': 'Last frame',
  'custom.submit': 'Split across machines',

  // Summaries
  'summary.title': 'Summarize a document',
  'summary.needsModel': 'Needs a machine with a local AI model; see {ask} above.',
  'summary.intro':
    'Open a PDF or text file. Its parts are summarized at the same time on {machines}, then combined into one summary. Only the text leaves this screen.',
  'summary.file': 'Document (.pdf, .txt, .md)',
  'summary.start': 'Summarize on {machines}',
  'summary.reading': 'Reading…',
  'summary.readingPage': 'Reading page {done} of {total}…',
  'summary.aboutParts': 'about {parts}',
  'summary.readFailed': 'Could not read {name}: {error}',
  'summary.meta': '{chars} in {parts}',
  'summary.step': 'Step {step}: {label}',
  'summary.stepOn': 'Step {step}: {label} on {machines}',
  'summary.result': 'Summary',
  'summary.failed': 'Failed',
  'sumstep.parts': 'Summarizing {parts}',
  'sumstep.final': 'Writing the final summary',
  'sumstep.combine': 'Combining {groups} of summaries',
  'role.parts': 'Parts of “{title}”',
  'role.final': 'Final summary of “{title}”',
  'role.combine': 'Combining summaries of “{title}”',
  'doc.empty': 'No text found. A scanned PDF is only pictures of pages; it needs text recognition (OCR) first.',
  'doc.tooLong': 'The document has {chars} characters; at most {max} can be summarized.',
  'doc.tooBig': 'The file is larger than {mb} MB.',

  // Jobs
  'state.pending': 'Queued',
  'state.running': 'Running',
  'state.done': 'Done',
  'state.failed': 'Failed',
  'jobs.label': 'Jobs',
  'jobs.empty': 'No jobs yet. Start one above and it will be split across the machines.',
  'jobs.earlier': 'Earlier jobs ({count})',
  'job.name': 'Job {id}',
  'job.meta': '{units} in {pieces} on {machines}',
  'job.noMachine': 'No machine yet',
  'job.barLabel': 'Job {id}: {pct} of {total} done across {machines}',
  'job.answers': 'Answers ({done} of {total})',
  'move.label': 'Moved to another machine:',
  'move.moved': '{label} {range} from {from} → {to}',
  'move.leftWho': '{name} left:',
  'move.left': '{who} {range} wait for another machine…',
  'move.from': '→ moved from {name}',

  // Errors the dashboard itself explains (the master's own text is shown as sent)
  'err.gone': 'That machine is no longer waiting; it may have disconnected.',
  'err.notPaired': 'That machine is not paired.',
  'err.token': 'The admin token was not accepted.',
  'err.status': 'master answered {status}',
  'err.notConnected': 'Not connected to a master.',
} as const satisfies Record<string, Message>;

export type MessageKey = keyof typeof en;

const ko = {
  'conn.connecting': '연결 중…',
  'conn.live': '연결됨',
  'conn.offline': '마스터에 연결할 수 없어 다시 시도하는 중…',
  'conn.unauthorized': '관리 토큰이 맞지 않습니다',
  'conn.disconnect': '연결 끊기',
  'kpi.label': '클러스터 요약',
  'kpi.online': '연결된 기기',
  'kpi.running': '실행 중인 조각',
  'kpi.cpu': '평균 CPU',

  'connect.title': '마스터에 연결',
  'connect.intro':
    '한 기기에서 {master}를 시작한 뒤, 그 주소와 처음 시작할 때 출력하는 관리 토큰을 입력하세요(데이터 폴더의 {file} 파일에도 저장됩니다).',
  'connect.address': '마스터 주소',
  'connect.badAddress': '주소 형식이 아닙니다.',
  'connect.token': '관리 토큰',
  'connect.submit': '연결',

  'count.machine': '기기 {count}대',
  'count.piece': '조각 {count}개',
  'count.frame': '프레임 {count}개',
  'count.question': '질문 {count}개',
  'count.part': '부분 {count}개',
  'count.group': '묶음 {count}개',
  'count.summary': '요약 {count}개',
  'count.page': '{count}쪽',
  'count.char': '{count}자',
  'count.step': '{count}단계',
  'count.core': '{count}코어',
  'unit.frame': '프레임',
  'unit.question': '질문',
  'unit.part': '부분',
  'unit.group': '묶음',
  'unit.summary': '요약',

  'common.cancel': '취소',
  'common.copy': '복사',
  'common.copied': '복사됨',
  'common.starting': '시작하는 중…',
  'common.working': '처리 중…',

  'map.title': '클러스터',
  'map.master': '마스터',
  'map.summary': '마스터에 기기 {count}대 중 {online}대 연결, {working}대 작업 중.',
  'map.none': '아직 짝지은 기기 없음',
  'node.offline': '연결 끊김',
  'node.idle': '대기 중',
  'node.working': '작업 중',
  'node.progress': '{name}의 {range}',
  'node.waitingReport': '첫 보고를 기다리는 중…',
  'node.memoryOf': '{total} 중 {used}',
  'mem.high': '높음',
  'mem.critical': '위험',
  'machines.title': '기기',
  'machines.online': '({count}대 연결)',
  'machines.none': '아직 연결된 기기가 없습니다. {add}를 보세요.',
  'col.machine': '기기',
  'col.status': '상태',
  'col.work': '맡은 조각',
  'col.cpu': 'CPU',
  'col.memory': '메모리',

  'pending.title': '승인 대기',
  'pending.hint': '같은 코드가 보이는 기기만 승인하세요. 코드는 그 기기의 에이전트가 시작할 때 출력하며, {command} 명령으로도 볼 수 있습니다.',
  'pending.codeLabel': '확인 코드 {code}',
  'pending.confirm': '코드 {code}인 {name} 기기를 승인할까요? 먼저 그 기기에서 코드를 확인하세요.',
  'pending.confirmFrom': '{address}에서 온, 코드 {code}인 {name} 기기를 승인할까요? 먼저 그 기기에서 코드를 확인하세요.',
  'pending.approveAnyway': '그래도 승인',
  'pending.approve': '승인',
  'pending.ignore': '무시',
  'flag.sameCode': '승인 대기 중인 다른 기기가 같은 코드를 보여 줍니다.',
  'flag.sameId': '승인 대기 중인 다른 기기가 같은 id를 씁니다.',
  'flag.sameName': '승인 대기 중인 다른 기기와 이름이 같습니다.',
  'flag.pairedName': '짝지은 기기 중에 이미 같은 이름이 있습니다.',
  'paired.title': '짝지은 기기 ({count})',
  'paired.online': '연결됨',
  'paired.offline': '연결 끊김',
  'paired.remove': '제거',
  'paired.removing': '제거하는 중…',
  'paired.removeForGood': '완전히 제거',
  'paired.explain':
    '{name} 기기를 제거하면 바로 연결이 끊기고, 그 뒤로는 그 기기의 키를 받지 않습니다. 다시 추가하려면 그 기기에서 {reset} 명령을 실행하고 여기서 승인하세요. 그 기기를 {token} 옵션으로 시작했다면 관리 토큰도 갖고 있으니 {rotate} 명령으로 토큰을 바꾸세요.',

  'add.title': '기기 추가',
  'add.step1': '다른 컴퓨터에 Nodeloom을 설치하고(이 컴퓨터와 같은 설치 파일) 실행하세요.',
  'add.step2': '{join} 버튼을 누르세요. 같은 네트워크라면 이 클러스터를 스스로 찾습니다.',
  'add.step3': '이 화면의 {waiting}에 6자리 코드와 함께 나타납니다. 그 컴퓨터의 코드와 같으면 승인하세요.',
  'add.trouble': '나타나지 않나요?',
  'add.network': '두 컴퓨터가 같은 네트워크에 있어야 합니다(게스트 Wi-Fi나 VPN은 안 됩니다).',
  'add.firewall':
    'Windows에서는 물어보는 창이 뜨면 이 컴퓨터와 다른 컴퓨터 모두에서 Nodeloom을 방화벽에 허용하세요. 네트워크가 {pub}으로 되어 있으면 {priv}으로 바꾸세요.',
  'add.public': '공용',
  'add.private': '개인',
  'add.manual': '다른 컴퓨터에서 {enter} 버튼을 누르고 다음을 입력하세요:',
  'add.cli': '라즈베리 파이, Jetson, 서버(명령줄)에서는:',

  'start.title': '작업 시작',
  'start.noMachine': '먼저 기기를 추가하세요.',
  'start.noFake': '테스트 실행기가 있는 기기가 없습니다. 에이전트를 {flag} 옵션으로 시작하세요.',
  'start.demoHint': '데모 작업: 테스트 프레임 600개를 {machines}에 나눠 약 {seconds}초. 기다리기만 하는 테스트로, 실제로 렌더링하지 않습니다.',
  'start.demo': '데모 작업',
  'ai.title': '로컬 AI에 질문',
  'ai.none': '아직 로컬 LLM을 실행하는 기기가 없습니다. 기기에 {ollama}와 모델을 설치하고(예: {command}) 그 기기에서 Nodeloom을 다시 시작하세요.',
  'ai.hint': '{questions}를 {machines}에 나눠, 기기마다 자기 모델로 답합니다.',
  'ai.model': '모델',
  'ai.questions': '질문 ({count})',
  'ai.perLine': '한 줄에 질문 하나',
  'ai.ask': 'AI에게 묻기',
  'prompt.none': '질문을 한 줄에 하나씩, 하나 이상 쓰세요.',
  'prompt.tooMany': '질문은 최대 {max}개입니다.',
  'prompt.tooLong': '{n}번 질문이 {max}자를 넘습니다.',
  'custom.title': '직접 설정',
  'custom.runner': '실행기',
  'custom.blender': 'Blender 렌더',
  'custom.fake': '테스트(기다리기만 함)',
  'custom.addFirst': '먼저 기기를 추가하세요',
  'custom.blendFile': '.blend 파일(각 기기 작업 폴더 기준 상대 경로)',
  'custom.millis': '프레임당 밀리초',
  'custom.first': '첫 프레임',
  'custom.last': '마지막 프레임',
  'custom.submit': '기기들에 나눠 실행',

  'summary.title': '문서 요약',
  'summary.needsModel': '로컬 AI 모델이 있는 기기가 필요합니다. 위의 {ask} 안내를 보세요.',
  'summary.intro':
    'PDF나 텍스트 파일을 여세요. 문서를 나눈 부분을 {machines}에서 동시에 요약한 뒤 하나로 합칩니다. 이 화면 밖으로는 글자만 보냅니다.',
  'summary.file': '문서(.pdf, .txt, .md)',
  'summary.start': '{machines}에서 요약',
  'summary.reading': '읽는 중…',
  'summary.readingPage': '{total}쪽 중 {done}쪽 읽는 중…',
  'summary.aboutParts': '약 {parts}',
  'summary.readFailed': '{name} 파일을 읽지 못했습니다: {error}',
  'summary.meta': '{chars} · {parts}',
  'summary.step': '{step}단계: {label}',
  'summary.stepOn': '{step}단계: {machines}에서 {label}',
  'summary.result': '요약',
  'summary.failed': '실패',
  'sumstep.parts': '{parts} 요약 중',
  'sumstep.final': '최종 요약 쓰는 중',
  'sumstep.combine': '요약 {groups} 합치는 중',
  'role.parts': '“{title}”의 부분',
  'role.final': '“{title}”의 최종 요약',
  'role.combine': '“{title}”의 요약 합치기',
  'doc.empty': '글자를 찾지 못했습니다. 스캔한 PDF는 쪽마다 그림일 뿐이라 먼저 글자 인식(OCR)이 필요합니다.',
  'doc.tooLong': '문서가 {chars}자입니다. 최대 {max}자까지 요약할 수 있습니다.',
  'doc.tooBig': '파일이 {mb}MB보다 큽니다.',

  'state.pending': '대기',
  'state.running': '실행 중',
  'state.done': '완료',
  'state.failed': '실패',
  'jobs.label': '작업',
  'jobs.empty': '아직 작업이 없습니다. 위에서 시작하면 기기들에 나눠 처리합니다.',
  'jobs.earlier': '이전 작업 ({count})',
  'job.name': '작업 {id}',
  'job.meta': '{units} · {pieces} · {machines}',
  'job.noMachine': '아직 기기 없음',
  'job.barLabel': '작업 {id}: {total} 중 {pct} 완료, {machines}',
  'job.answers': '답변 ({total}개 중 {done}개)',
  'move.label': '다른 기기로 넘어감:',
  'move.moved': '{label} {range}, {from} → {to}',
  'move.leftWho': '{name} 연결 끊김:',
  'move.left': '{who} {range}, 다른 기기를 기다리는 중…',
  'move.from': '→ {name}에서 넘어옴',

  'err.gone': '그 기기는 더 이상 승인을 기다리지 않습니다. 연결이 끊겼을 수 있습니다.',
  'err.notPaired': '짝지은 기기가 아닙니다.',
  'err.token': '관리 토큰이 받아들여지지 않았습니다.',
  'err.status': '마스터 응답 {status}',
  'err.notConnected': '마스터에 연결되어 있지 않습니다.',
} satisfies Record<MessageKey, Message>;

export const MESSAGES: Record<Lang, Record<MessageKey, Message>> = { en, ko };

export type Translate = (key: MessageKey, params?: Params) => string;

const numberLocale: Record<Lang, string> = { en: 'en-US', ko: 'ko-KR' };

/**
 * The text of `key` in `lang`. A plural message picks its form by
 * `params.count`. Numbers are shown with the language's digit grouping.
 * A placeholder with no parameter stays as `{name}`, for `splitSlots`.
 */
export function translate(lang: Lang, key: MessageKey, params: Params = {}): string {
  const message = MESSAGES[lang][key];
  const text =
    typeof message === 'string'
      ? message
      : message[new Intl.PluralRules(lang).select(Number(params.count ?? 0)) === 'one' ? 'one' : 'other'];
  return text.replace(/\{(\w+)\}/g, (whole, name: string) => {
    const value = params[name];
    if (value === undefined) return whole;
    return typeof value === 'number' ? value.toLocaleString(numberLocale[lang]) : value;
  });
}

/** `translate` with the language fixed, for pure functions that build text. */
export function translator(lang: Lang): Translate {
  return (key, params) => translate(lang, key, params);
}

/** Things a job is counted in; each has `count.*` and `unit.*` messages. */
export type UnitName = 'frame' | 'question' | 'part' | 'group' | 'summary';

/** "frames" / "프레임": the unit before a range like 201–400. */
export function unitWord(t: Translate, unit: UnitName, count: number): string {
  return t(`unit.${unit}`, { count });
}

/** A text with `{slot}` places left for markup, cut into text and slot names. */
export type Segment = { text: string } | { slot: string };

export function splitSlots(text: string): Segment[] {
  const out: Segment[] = [];
  let last = 0;
  for (const m of text.matchAll(/\{(\w+)\}/g)) {
    if (m.index > last) out.push({ text: text.slice(last, m.index) });
    out.push({ slot: m[1]! });
    last = m.index + m[0].length;
  }
  if (last < text.length) out.push({ text: text.slice(last) });
  return out;
}

/**
 * An error the dashboard explains itself, shown in the current language.
 * Its `message` stays English for logs and for callers that only read it.
 */
export class MessageError extends Error {
  constructor(
    readonly key: MessageKey,
    readonly params: Params = {},
  ) {
    super(translate('en', key, params));
  }
}

/** What to show for a caught error. A master's own explanation is shown as sent. */
export function errorText(error: unknown, t: Translate): string {
  if (error instanceof MessageError) return t(error.key, error.params);
  return error instanceof Error ? error.message : String(error);
}

export const LANG_KEY = 'nodeloom.lang';

export function isLang(value: unknown): value is Lang {
  return value === 'en' || value === 'ko';
}

/**
 * The language to start in: the saved choice, else Korean when the browser
 * prefers it, else English. Anything odd in storage is ignored.
 */
export function initialLang(saved: string | null, browser: readonly string[]): Lang {
  if (isLang(saved)) return saved;
  return browser[0]?.toLowerCase().startsWith('ko') ? 'ko' : 'en';
}

/** The saved choice, or null when there is none or storage is blocked. */
export function loadLang(storage: () => Pick<Storage, 'getItem'>): string | null {
  try {
    return storage().getItem(LANG_KEY);
  } catch {
    return null;
  }
}

/** Remembers the choice; a blocked storage only means it is not remembered. */
export function saveLang(storage: () => Pick<Storage, 'setItem'>, lang: Lang): void {
  try {
    storage().setItem(LANG_KEY, lang);
  } catch {
    /* ignore */
  }
}
