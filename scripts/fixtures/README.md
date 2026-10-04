# 首次配置的合成资源

`onboarding.scs.base64` 是完全合成的 HashFS v2 资源，仅供桌面测试。它只包含一个配件定义，不含游戏资产或玩家信息；不会进入发布 ZIP。

解码后 SHA256：`f7160d049b6104616373346bfd9fbf01d78a9f2a43985ef9a78521061e7ed9a8`。

源文件路径：`def/vehicle/truck/synthetic/engine/test.sii`，内容为：

```text
SiiNunit
{
accessory_engine_data : synthetic.engine {
 name: "Synthetic engine"
 torque: 1000
}
}
```

由 [SCS 官方 Game Archive Packer](https://modding.scssoft.com/wiki/Documentation/Tools/Game_Archive_Packer) 生成：

```powershell
scs_packer.exe create synthetic.scs -root source
[Convert]::ToBase64String([IO.File]::ReadAllBytes('synthetic.scs'))
```

生成时使用官方 `https://download.eurotrucksimulator2.com/scs_packer_1_55.zip`：ZIP SHA256 为 `82f716a0261d1fd1582f2df30f536612a52ac267c74f1fd3c0f93115bf49d1ce`，工具 SHA256 为 `aa9cdbc168f372a219c1fd017eea97cc221070fcf3991813b5c3e9124aad10e5`。工具不随仓库提供；运行已有测试样本不需要 packer。
