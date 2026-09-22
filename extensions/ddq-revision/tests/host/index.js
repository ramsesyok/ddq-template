/**
 * 実拡張ホストでの検証（vitest では届かない部分）。
 *
 *   node tests/host/run.mjs
 *
 * VSCode を `--extensionDevelopmentPath` / `--extensionTestsPath` で起動し、この `run()` が
 * 本物の VSCode API の上で走る。見るのは 2 つの画面:
 *
 * ラベル一覧:
 *   1. コマンドでパネルが開くこと
 *   2. `ddq tag list --json` の結果から作った編集指示を WorkspaceEdit で当てられること
 *      （日本語の行でも位置がずれないこと）
 *   3. 当てた直後に Undo で元へ戻せること
 *
 * 改訂履歴（実物の ddq があるときだけ）:
 *   4. `ddq rev diff --write` が作った yml が Custom Editor で開くこと
 *   5. 画面で書いたメモが ddq の読める形で文書に入り、`ddq rev build` が表に出すこと
 *   6. 差分ボタンで VSCode の差分エディタが開くこと（左 git: / 右 作業ツリー）
 */
const assert = require('node:assert');
const { execFileSync } = require('node:child_process');
const fs = require('node:fs');
const path = require('node:path');
const vscode = require('vscode');

const log = [];

function say(what) {
    log.push(`OK   ${what}`);
}

/** 1 行の見出しに候補ラベルを当てる行を作る。 */
function rowsFor(list, file) {
    return list.items
        .filter((item) => item.suggested && item.file === file)
        .map((item) => ({ item, label: item.suggested, checked: true }));
}

async function run() {
    const docs = process.env.DOCS_DIR || path.join(vscode.workspace.workspaceFolders[0].uri.fsPath, 'docs');
    const ddq = process.env.DDQ_BIN;
    assert.ok(ddq && fs.existsSync(ddq), 'DDQ_BIN に ddq のパスを渡してください');

    // 1) 一覧のパネルが開く
    await vscode.commands.executeCommand('ddqRevision.tags', vscode.Uri.file(docs));
    await new Promise((r) => setTimeout(r, 2000));
    const tab = vscode.window.tabGroups.activeTabGroup.activeTab;
    assert.ok(tab, 'タブが開いていない');
    assert.ok(tab.input instanceof vscode.TabInputWebview, `Webview ではない: ${tab.input}`);
    assert.match(tab.label, /ラベル一覧/, `タブ名が違う: ${tab.label}`);
    say(`一覧のパネルが開く（${tab.label}）`);

    // 2) ddq の一覧から書き戻す
    // 拡張と同じ起動のしかた（Windows の .cmd は cmd /c 経由でないと起動できない）
    const argv = ['tag', 'list', docs, '--json'];
    const [file, args] =
        process.platform === 'win32' && /\.(cmd|bat)$/i.test(ddq)
            ? ['cmd.exe', ['/c', ddq, ...argv]]
            : [ddq, argv];
    const list = JSON.parse(execFileSync(file, args, { encoding: 'utf8' }));
    const target = 'chapters/01-overview/index.qmd';
    const rows = rowsFor(list, target);
    assert.ok(rows.length >= 1, 'ラベル未付与の見出しが無い');
    const before = fs.readFileSync(path.join(docs, target), 'utf8');

    const result = await vscode.commands.executeCommand(
        'ddqRevision.internal.applyRows',
        docs,
        list,
        rows
    );
    assert.deepStrictEqual(result.errors, [], `検査で弾かれた: ${JSON.stringify(result.errors)}`);
    assert.deepStrictEqual(result.skipped, [], `飛ばされた: ${result.skipped}`);
    assert.strictEqual(result.applied, rows.length);
    say(`${result.applied} 件を WorkspaceEdit で書き戻した`);

    const doc = await vscode.workspace.openTextDocument(vscode.Uri.file(path.join(docs, target)));
    const after = doc.getText();
    assert.notStrictEqual(after, before, '本文が変わっていない');
    for (const row of rows) {
        const line = doc.lineAt(row.item.line - 1).text;
        assert.ok(line.includes(row.label), `${row.item.line} 行目に ${row.label} が無い: ${line}`);
        // 日本語の見出しでも、文言のあとに正しく入っていること
        assert.ok(
            row.item.title === null || line.startsWith(`${'#'.repeat(row.item.level ?? 1)} ${row.item.title}`),
            `見出しの文言が壊れている: ${line}`
        );
    }
    say('日本語の見出しでも挿入位置がずれない');

    // 未保存のバッファに当たっている（保存は人がする）
    assert.ok(doc.isDirty, '未保存のバッファに当たっていない');
    say('未保存のバッファに当たる（保存は人が行う）');

    // 3) Undo で戻る
    await vscode.window.showTextDocument(doc);
    await vscode.commands.executeCommand('undo');
    await new Promise((r) => setTimeout(r, 500));
    assert.strictEqual(doc.getText(), before, 'Undo で戻らない');
    say('Ctrl+Z ひと押しで元に戻る');

    // 4) 改訂履歴の編集画面（Custom Editor）。実物の ddq が要るので、差し替えのときは飛ばす
    if (process.env.DDQ_IS_REAL === '1') {
        await revisionEditor(docs, ddq);
    } else {
        log.push('--   改訂履歴の編集画面は実物の ddq が要るので飛ばした');
    }

    fs.writeFileSync(path.join(process.env.HOST_OUT, 'host-result.txt'), log.join('\n'), 'utf8');
}

/**
 * 改訂履歴の編集画面を通して見る。
 *   1. 差分を取って yml ができる（ddq rev diff --write）
 *   2. その yml が Custom Editor で開く
 *   3. 画面から書いたメモが文書に入り、保存すると ddq が読める形で残る
 *   4. 差分ボタンが VSCode の差分エディタを開く（左が git:、右が作業ツリー）
 */
async function revisionEditor(docs, ddq) {
    const repo = path.dirname(docs);
    const git = (args) => execFileSync('git', args, { cwd: repo, encoding: 'utf8' });

    // 改訂履歴はラベルが前提（ラベルの無い見出しは追えない）。ここまでの手順では
    // 書き戻しを Undo で戻しているので、まず実際にラベルを付けてから始める。
    execFileSync(ddq, ['tag', 'apply', docs, '--all'], { encoding: 'utf8' });
    git(['init', '-q', '.']);
    git(['config', 'user.email', 't@example.com']);
    git(['config', 'user.name', 'test']);
    git(['add', '-A']);
    git(['commit', '-qm', '初版']);
    git(['tag', 'rev-A']);

    // 本文を直してから差分を取る
    const target = path.join(docs, 'chapters', '01-overview', 'index.qmd');
    fs.writeFileSync(
        target,
        fs.readFileSync(target, 'utf8').replace('導入の本文。', '導入の本文を直した。')
    );
    execFileSync(ddq, ['rev', 'diff', docs, '--write'], { encoding: 'utf8' });

    const yml = path.join(docs, 'revisions', 'rev-B.yml');
    assert.ok(fs.existsSync(yml), 'rev-B.yml ができていない');
    say('ddq rev diff --write で改訂ファイルができる');

    await vscode.commands.executeCommand(
        'vscode.openWith',
        vscode.Uri.file(yml),
        'ddqRevision.editor'
    );
    await new Promise((r) => setTimeout(r, 2000));
    const tab = vscode.window.tabGroups.activeTabGroup.activeTab;
    assert.ok(
        tab.input instanceof vscode.TabInputCustom,
        `Custom Editor ではない: ${tab.input && tab.input.constructor.name}`
    );
    assert.strictEqual(tab.input.viewType, 'ddqRevision.editor');
    say(`改訂ファイルが編集画面で開く（${tab.label}）`);

    // 画面から書いたのと同じことを内部コマンドで行い、文書 → 保存 → ddq が読める形を見る
    const doc = await vscode.workspace.openTextDocument(vscode.Uri.file(yml));
    const written = await vscode.commands.executeCommand(
        'ddqRevision.internal.writeNote',
        doc.uri.toString(),
        '導入の説明を補足した。'
    );
    assert.ok(written, 'メモを書けなかった');
    assert.ok(doc.isDirty, '未保存のバッファに入っていない');
    await doc.save();
    assert.ok(
        /note: \|\r?\n\s+導入の説明を補足した。/.test(fs.readFileSync(yml, 'utf8')),
        'ddq の読める形でメモが入っていない'
    );
    say('画面で書いたメモが ddq の読める形で文書に入る');

    // ddq が読めること（表が作れること）で往復を確かめる
    const out = execFileSync(ddq, ['rev', 'build', docs], { encoding: 'utf8' });
    const history = fs.readFileSync(path.join(docs, 'revisions', 'history.qmd'), 'utf8');
    assert.ok(history.includes('導入の説明を補足した。'), `表にメモが出ていない: ${out}`);
    say('ddq rev build がそのメモを表に出す');

    // 差分エディタ（左が git:、右が作業ツリー）
    await vscode.commands.executeCommand('ddqRevision.internal.openDiff', doc.uri.toString(), 0);
    await new Promise((r) => setTimeout(r, 1500));
    const diffTab = vscode.window.tabGroups.activeTabGroup.activeTab;
    assert.ok(
        diffTab.input instanceof vscode.TabInputTextDiff,
        `差分エディタではない: ${diffTab.input && diffTab.input.constructor.name}`
    );
    assert.strictEqual(diffTab.input.original.scheme, 'git');
    assert.strictEqual(diffTab.input.modified.scheme, 'file');
    say('差分ボタンで VSCode の差分エディタが開く（左 git: / 右 作業ツリー）');
}

module.exports.run = () =>
    run().catch((e) => {
        try {
            fs.writeFileSync(
                path.join(process.env.HOST_OUT, 'host-result.txt'),
                `${log.join('\n')}\nNG   ${(e && e.stack) || e}`,
                'utf8'
            );
        } catch {
            /* ignore */
        }
        throw e;
    });
