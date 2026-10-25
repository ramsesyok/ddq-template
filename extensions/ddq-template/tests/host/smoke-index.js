/**
 * 同梱版のスモークの中身（smoke.mjs が、展開した VSIX を拡張として VSCode を起動して走らせる）。
 *
 *   1. 設定 ddqTemplate.ddqPath が空で PATH にも ddq が無いとき、同梱の ddq に決まる
 *   2. その ddq で設計書リポジトリを作れる（ddq init。Quarto があれば疎通確認の render まで）
 *   3. 作った執筆フォルダの HTML を作れる（ddq html）
 */
const assert = require('node:assert');
const fs = require('node:fs');
const path = require('node:path');
const vscode = require('vscode');

const log = [];

function say(what) {
    log.push(`OK   ${what}`);
}

async function run() {
    // 内部コマンドは package.json に出していないので、呼んでも拡張は起動しない。先に起動しておく
    const extension = vscode.extensions.getExtension('ramsesyok.ddq-template');
    assert.ok(extension, '拡張 ramsesyok.ddq-template が読み込まれていない');
    await extension.activate();

    const configured = vscode.workspace.getConfiguration('ddqTemplate').get('ddqPath');
    assert.ok(!configured, `設定 ddqTemplate.ddqPath が入っている: ${configured}`);
    const located = await vscode.commands.executeCommand('ddqTemplate.internal.locate');
    assert.strictEqual(located.source, 'bundled', `同梱に決まらない: ${JSON.stringify(located)}`);
    assert.strictEqual(
        path.resolve(located.command).toLowerCase(),
        path.resolve(process.env.BUNDLED_DDQ).toLowerCase(),
        '展開した VSIX の ddq.exe ではない'
    );
    say(`設定も PATH も無しで同梱の ddq に決まる（${located.command}）`);

    const repo = path.join(process.env.HOST_OUT, 'smoke-repo');
    const init = await vscode.commands.executeCommand('ddqTemplate.internal.init', repo, 'docs');
    assert.strictEqual(init, 'ok', `init が通らない: ${init}`);
    const docs = path.join(repo, 'docs');
    assert.ok(fs.existsSync(path.join(docs, '_quarto.yml')), '_quarto.yml が無い');
    say('同梱の ddq で設計書リポジトリを作れる（ddq init）');

    const html = await vscode.commands.executeCommand('ddqTemplate.internal.build', 'html', docs);
    assert.strictEqual(html, 'ok', `html が通らない: ${html}`);
    assert.ok(fs.existsSync(path.join(docs, '_book', 'index.html')), '_book/index.html が無い');
    say('作った執筆フォルダの HTML を作れる（ddq html）');

    fs.writeFileSync(path.join(process.env.HOST_OUT, 'host-result.txt'), log.join('\n'), 'utf8');
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
