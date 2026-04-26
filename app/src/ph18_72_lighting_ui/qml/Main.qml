import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

ApplicationWindow {
    id: root
    width: 1280
    height: 820
    visible: true
    title: "PH18-72 Lighting Test Console"
    color: "#0f1317"

    property string selectedKeyboardKey: "5"
    property string selectedKeyboardLabel: "5/%"
    property string selectedMagkey: "w"
    property string selectedMagkeyLabel: "W"
    property string selectedCoverSegment: "all"
    property string selectedCoverSegmentLabel: "All"

    readonly property color testingButtonColor: "#f0b429"
    readonly property color testingButtonBorder: "#d39c18"
    readonly property color testingButtonText: "#1d1400"
    readonly property var stubbornKeys: ["5", "semicolon", "keypad_6", "arrow_down"]

    readonly property var panels: [
        { name: "Main Keyboard", status: "Experimental", color: "#c27803" },
        { name: "MagKey 3.0", status: "Experimental", color: "#c27803" },
        { name: "Cover Logo", status: "Experimental", color: "#c27803" },
        { name: "Base Logo", status: "In Development", color: "#c27803" },
        { name: "Infinity Mirror", status: "In Development", color: "#c27803" }
    ]

    readonly property var keyboardRows: [
        [
            { label: "Esc", name: "esc" }, { label: "1", name: "1" }, { label: "2", name: "2" },
            { label: "3", name: "3" }, { label: "4", name: "4" }, { label: "5/%", name: "5" },
            { label: "6", name: "6" }, { label: "7", name: "7" }, { label: "8", name: "8" },
            { label: "9", name: "9" }, { label: "0", name: "0" }, { label: "-", name: "minus" },
            { label: "=", name: "equal" }, { label: "Bksp", name: "backspace" }
        ],
        [
            { label: "Tab", name: "tab" }, { label: "Q", name: "q" }, { label: "W", name: "w" },
            { label: "E", name: "e" }, { label: "R", name: "r" }, { label: "T", name: "t" },
            { label: "Y", name: "y" }, { label: "U", name: "u" }, { label: "I", name: "i" },
            { label: "O", name: "o" }, { label: "P", name: "p" }, { label: "[", name: "left_bracket" },
            { label: "]", name: "right_bracket" }, { label: "\\", name: "backslash" }
        ],
        [
            { label: "Caps", name: "caps_lock" }, { label: "A", name: "a" }, { label: "S", name: "s" },
            { label: "D", name: "d" }, { label: "F", name: "f" }, { label: "G", name: "g" },
            { label: "H", name: "h" }, { label: "J", name: "j" }, { label: "K", name: "k" },
            { label: "L", name: "l" }, { label: ";/:", name: "semicolon" }, { label: "'", name: "apostrophe" },
            { label: "Enter", name: "enter" }
        ],
        [
            { label: "Shift", name: "left_shift" }, { label: "Z", name: "z" }, { label: "X", name: "x" },
            { label: "C", name: "c" }, { label: "V", name: "v" }, { label: "B", name: "b" },
            { label: "N", name: "n" }, { label: "M", name: "m" }, { label: ",", name: "comma" },
            { label: ".", name: "period" }, { label: "/", name: "slash" }, { label: "RShift", name: "right_shift" }
        ],
        [
            { label: "Ctrl", name: "left_ctrl" }, { label: "Win", name: "left_windows" }, { label: "Alt", name: "left_alt" },
            { label: "Space", name: "space" }, { label: "RAlt", name: "right_alt" }, { label: "Menu", name: "menu" },
            { label: "Left", name: "arrow_left" }, { label: "Down", name: "arrow_down" }, { label: "Right", name: "arrow_right" },
            { label: "Up", name: "arrow_up" }
        ],
        [
            { label: "Num", name: "keypad_num_lock" }, { label: "/", name: "keypad_divide" }, { label: "*", name: "keypad_multiply" },
            { label: "-", name: "keypad_minus" }, { label: "7", name: "keypad_7" }, { label: "8", name: "keypad_8" },
            { label: "9", name: "keypad_9" }, { label: "4", name: "keypad_4" }, { label: "5", name: "keypad_5" },
            { label: "6", name: "keypad_6" }, { label: "1", name: "keypad_1" }, { label: "2", name: "keypad_2" },
            { label: "3", name: "keypad_3" }, { label: "0", name: "keypad_0" }, { label: ".", name: "keypad_decimal" },
            { label: "Enter", name: "keypad_enter" }
        ]
    ]

    readonly property var magkeyButtons: [
        { label: "W", name: "w" },
        { label: "A", name: "a" },
        { label: "S", name: "s" },
        { label: "D", name: "d" }
    ]

    Rectangle {
        anchors.fill: parent
        gradient: Gradient {
            GradientStop { position: 0.0; color: "#141a20" }
            GradientStop { position: 1.0; color: "#0a0d10" }
        }
    }

    RowLayout {
        anchors.fill: parent
        anchors.margins: 20
        spacing: 18

        Rectangle {
            Layout.preferredWidth: 280
            Layout.fillHeight: true
            radius: 8
            color: "#161c22"
            border.color: "#2a323b"

            ColumnLayout {
                anchors.fill: parent
                anchors.margins: 18
                spacing: 14

                Label {
                    text: "Controllers"
                    color: "#f3f4f6"
                    font.pixelSize: 24
                    font.bold: true
                }

                Repeater {
                    model: root.panels

                    delegate: Rectangle {
                        required property var modelData
                        Layout.fillWidth: true
                        height: 76
                        radius: 8
                        color: lightingUiModel.selectedPanel === modelData.name ? "#222b34" : "#10151a"
                        border.color: lightingUiModel.selectedPanel === modelData.name ? "#5b6b7c" : "#2a323b"

                        MouseArea {
                            anchors.fill: parent
                            onClicked: lightingUiModel.selectedPanel = parent.modelData.name
                        }

                        RowLayout {
                            anchors.fill: parent
                            anchors.margins: 14
                            spacing: 12

                            Rectangle {
                                width: 12
                                height: 12
                                radius: 6
                                color: modelData.color
                            }

                            ColumnLayout {
                                Layout.fillWidth: true
                                spacing: 3

                                Label {
                                    text: modelData.name
                                    color: "#eef2f7"
                                    font.pixelSize: 17
                                    font.bold: true
                                }

                                Label {
                                    text: modelData.status
                                    color: "#b6c0cb"
                                    font.pixelSize: 13
                                }
                            }
                        }
                    }
                }

                Item { Layout.fillHeight: true }

                Button {
                    text: "Run Inventory"
                    Layout.fillWidth: true
                    onClicked: lightingUiModel.runInventory()
                }
            }
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.fillHeight: true
            radius: 8
            color: "#161c22"
            border.color: "#2a323b"

            ColumnLayout {
                anchors.fill: parent
                anchors.margins: 18
                spacing: 16

                RowLayout {
                    Layout.fillWidth: true

                    Label {
                        text: lightingUiModel.selectedPanel
                        color: "#f3f4f6"
                        font.pixelSize: 26
                        font.bold: true
                        Layout.fillWidth: true
                    }

                    Rectangle {
                        radius: 999
                        color: lightingUiModel.backendMode === "cargo" ? "#153b29" : "#3d2b12"
                        border.color: lightingUiModel.backendMode === "cargo" ? "#2c8b63" : "#c27803"
                        height: 32
                        width: 132

                        Label {
                            anchors.centerIn: parent
                            text: lightingUiModel.backendMode === "cargo" ? "Real Backend" : "Mock Backend"
                            color: "#f3f4f6"
                            font.pixelSize: 13
                        }
                    }
                }

                Rectangle {
                    Layout.fillWidth: true
                    height: 1
                    color: "#2a323b"
                }

                StackLayout {
                    Layout.fillWidth: true
                    Layout.preferredHeight: 430
                    currentIndex: {
                        let names = ["Main Keyboard", "MagKey 3.0", "Cover Logo", "Base Logo", "Infinity Mirror"]
                        return names.indexOf(lightingUiModel.selectedPanel)
                    }

                    Item {
                        ColumnLayout {
                            anchors.fill: parent
                            spacing: 14

                            RowLayout {
                                Layout.fillWidth: true
                                Layout.fillHeight: true
                                spacing: 18

                                Rectangle {
                                    Layout.fillWidth: true
                                    Layout.fillHeight: true
                                    radius: 6
                                    color: "#10151a"
                                    border.color: "#2a323b"

                                    ColumnLayout {
                                        anchors.fill: parent
                                        anchors.margins: 12
                                        spacing: 8

                                        Label {
                                            text: "Per-key Keyboard Test Grid"
                                            color: "#cfd7df"
                                            font.pixelSize: 16
                                        }

                                        Repeater {
                                            model: root.keyboardRows

                                            delegate: Row {
                                                required property var modelData
                                                spacing: 6

                                                Repeater {
                                                    model: modelData

                                                    delegate: Button {
                                                        required property var modelData
                                                        text: modelData.label
                                                        implicitHeight: 30
                                                        implicitWidth: Math.max(38, contentItem.implicitWidth + 12)

                                                        onClicked: {
                                                            root.selectedKeyboardKey = modelData.name
                                                            root.selectedKeyboardLabel = modelData.label
                                                        }

                                                        background: Rectangle {
                                                            radius: 5
                                                            color: root.selectedKeyboardKey === modelData.name ? "#31404e" : "#182028"
                                                            border.color: root.selectedKeyboardKey === modelData.name ? "#84a1bc" : "#2a323b"
                                                        }

                                                        contentItem: Label {
                                                            text: parent.text
                                                            color: "#eef2f7"
                                                            font.pixelSize: 11
                                                            horizontalAlignment: Text.AlignHCenter
                                                            verticalAlignment: Text.AlignVCenter
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }

                                Rectangle {
                                    Layout.preferredWidth: 240
                                    Layout.fillHeight: true
                                    radius: 6
                                    color: "#10151a"
                                    border.color: "#2a323b"

                                    ColumnLayout {
                                        anchors.fill: parent
                                        anchors.margins: 12
                                        spacing: 10

                                        Label {
                                            text: "Selected: " + root.selectedKeyboardLabel
                                            color: "#eef2f7"
                                            font.pixelSize: 16
                                            font.bold: true
                                        }

                                        Label { text: "R"; color: "#cfd7df"; font.pixelSize: 13 }
                                        SpinBox { id: keyboardRed; from: 0; to: 255; value: 0; editable: true }
                                        Label { text: "G"; color: "#cfd7df"; font.pixelSize: 13 }
                                        SpinBox { id: keyboardGreen; from: 0; to: 255; value: 0; editable: true }
                                        Label { text: "B"; color: "#cfd7df"; font.pixelSize: 13 }
                                        SpinBox { id: keyboardBlue; from: 0; to: 255; value: 255; editable: true }

                                        Button {
                                            text: root.stubbornKeys.indexOf(root.selectedKeyboardKey) !== -1 ? "Apply Selected Key" : "Unsupported Key"
                                            Layout.fillWidth: true
                                            implicitWidth: 0
                                            Layout.maximumWidth: parent.width - 24
                                            enabled: root.stubbornKeys.indexOf(root.selectedKeyboardKey) !== -1
                                            onClicked: lightingUiModel.setKeyboardKeyColor(
                                                root.selectedKeyboardKey,
                                                keyboardRed.value,
                                                keyboardGreen.value,
                                                keyboardBlue.value
                                            )

                                            background: Rectangle {
                                                radius: 6
                                                color: root.testingButtonColor
                                                border.color: root.testingButtonBorder
                                            }

                                            contentItem: Label {
                                                text: parent.text
                                                color: root.testingButtonText
                                                font.pixelSize: 13
                                                font.bold: true
                                                elide: Text.ElideRight
                                                horizontalAlignment: Text.AlignHCenter
                                                verticalAlignment: Text.AlignVCenter
                                            }
                                        }

                                        Label {
                                            text: "Only the stubborn correction keys are enabled here for now: 5/%, ;/:, keypad 6, and Down."
                                            color: "#d9b86c"
                                            font.pixelSize: 12
                                            wrapMode: Text.Wrap
                                        }
                                    }
                                }
                            }

                            Label {
                                text: "Known-good whole keyboard commands"
                                color: "#cfd7df"
                                font.pixelSize: 16
                            }

                            RowLayout {
                                spacing: 10

                                Repeater {
                                    model: [
                                        { label: "Blue", fn: function() { lightingUiModel.setMainKeyboardBlue() } },
                                        { label: "Red", fn: function() { lightingUiModel.setMainKeyboardRed() } },
                                        { label: "Green", fn: function() { lightingUiModel.setMainKeyboardGreen() } }
                                    ]

                                    delegate: Button {
                                        required property var modelData
                                        text: modelData.label
                                        onClicked: modelData.fn()

                                        background: Rectangle {
                                            radius: 6
                                            color: root.testingButtonColor
                                            border.color: root.testingButtonBorder
                                        }

                                        contentItem: Label {
                                            text: parent.text
                                            color: root.testingButtonText
                                            font.pixelSize: 14
                                            font.bold: true
                                            horizontalAlignment: Text.AlignHCenter
                                            verticalAlignment: Text.AlignVCenter
                                        }
                                    }
                                }

                                Button { text: "Restore Known Good"; onClicked: lightingUiModel.restoreKnownGood() }
                            }

                            Label {
                                text: "Yellow means this command is still under live hardware validation. Whole-board red/green are experimental commit33 words with stubborn-key patching."
                                color: "#d9b86c"
                                font.pixelSize: 13
                            }
                        }
                    }

                    Item {
                        ColumnLayout {
                            anchors.fill: parent
                            spacing: 14

                            RowLayout {
                                Layout.fillWidth: true
                                Layout.fillHeight: true
                                spacing: 18

                                Rectangle {
                                    Layout.fillWidth: true
                                    Layout.fillHeight: true
                                    radius: 6
                                    color: "#10151a"
                                    border.color: "#2a323b"

                                    ColumnLayout {
                                        anchors.fill: parent
                                        anchors.margins: 12
                                        spacing: 10

                                        Label {
                                            text: "MagKey Test Grid"
                                            color: "#cfd7df"
                                            font.pixelSize: 16
                                        }

                                        Row {
                                            spacing: 10

                                            Repeater {
                                                model: root.magkeyButtons

                                                delegate: Button {
                                                    required property var modelData
                                                    text: modelData.label
                                                    implicitWidth: 64
                                                    implicitHeight: 48

                                                    onClicked: {
                                                        root.selectedMagkey = modelData.name
                                                        root.selectedMagkeyLabel = modelData.label
                                                    }

                                                    background: Rectangle {
                                                        radius: 6
                                                        color: root.selectedMagkey === modelData.name ? "#31404e" : "#182028"
                                                        border.color: root.selectedMagkey === modelData.name ? "#84a1bc" : "#2a323b"
                                                    }

                                                    contentItem: Label {
                                                        text: parent.text
                                                        color: "#eef2f7"
                                                        font.pixelSize: 16
                                                        font.bold: true
                                                        horizontalAlignment: Text.AlignHCenter
                                                        verticalAlignment: Text.AlignVCenter
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }

                                Rectangle {
                                    Layout.preferredWidth: 240
                                    Layout.fillHeight: true
                                    radius: 6
                                    color: "#10151a"
                                    border.color: "#2a323b"

                                    ColumnLayout {
                                        anchors.fill: parent
                                        anchors.margins: 12
                                        spacing: 10

                                        Label {
                                            text: "Selected: " + root.selectedMagkeyLabel
                                            color: "#eef2f7"
                                            font.pixelSize: 16
                                            font.bold: true
                                        }

                                        Label { text: "R"; color: "#cfd7df"; font.pixelSize: 13 }
                                        SpinBox { id: magkeyRed; from: 0; to: 255; value: 0; editable: true }
                                        Label { text: "G"; color: "#cfd7df"; font.pixelSize: 13 }
                                        SpinBox { id: magkeyGreen; from: 0; to: 255; value: 0; editable: true }
                                        Label { text: "B"; color: "#cfd7df"; font.pixelSize: 13 }
                                        SpinBox { id: magkeyBlue; from: 0; to: 255; value: 255; editable: true }

                                        Button {
                                            text: "Apply Selected MagKey"
                                            Layout.fillWidth: true
                                            implicitWidth: 0
                                            Layout.maximumWidth: parent.width - 24
                                            onClicked: lightingUiModel.setMagkeyKeyColor(
                                                root.selectedMagkey,
                                                magkeyRed.value,
                                                magkeyGreen.value,
                                                magkeyBlue.value
                                            )

                                            background: Rectangle {
                                                radius: 6
                                                color: root.testingButtonColor
                                                border.color: root.testingButtonBorder
                                            }

                                            contentItem: Label {
                                                text: parent.text
                                                color: root.testingButtonText
                                                font.pixelSize: 13
                                                font.bold: true
                                                elide: Text.ElideRight
                                                horizontalAlignment: Text.AlignHCenter
                                                verticalAlignment: Text.AlignVCenter
                                            }
                                        }

                                        Label {
                                            text: "MagKey single-key tests are interference-prone because the hardware shares slot lanes across neighboring keys."
                                            color: "#d9b86c"
                                            font.pixelSize: 12
                                            wrapMode: Text.Wrap
                                        }
                                    }
                                }
                            }

                            Label { text: "Known-safe MagKey commands"; color: "#cfd7df"; font.pixelSize: 16 }
                            RowLayout {
                                spacing: 10

                                Repeater {
                                    model: [
                                        { label: "Blue", fn: function() { lightingUiModel.setMagkeysBlue() } },
                                        { label: "Red", fn: function() { lightingUiModel.setMagkeysRed() } },
                                        { label: "Green", fn: function() { lightingUiModel.setMagkeysGreen() } }
                                    ]

                                    delegate: Button {
                                        required property var modelData
                                        text: modelData.label
                                        onClicked: modelData.fn()

                                        background: Rectangle {
                                            radius: 6
                                            color: root.testingButtonColor
                                            border.color: root.testingButtonBorder
                                        }

                                        contentItem: Label {
                                            text: parent.text
                                            color: root.testingButtonText
                                            font.pixelSize: 14
                                            font.bold: true
                                            horizontalAlignment: Text.AlignHCenter
                                            verticalAlignment: Text.AlignVCenter
                                        }
                                    }
                                }

                                Button { text: "Restore Known Good"; onClicked: lightingUiModel.restoreKnownGood() }
                            }

                            Label {
                                text: "Yellow means this command is still under live hardware validation."
                                color: "#d9b86c"
                                font.pixelSize: 13
                            }
                        }
                    }

                    Item {
                        ColumnLayout {
                            anchors.fill: parent
                            spacing: 14

                            RowLayout {
                                Layout.fillWidth: true
                                Layout.fillHeight: true
                                spacing: 18

                                Rectangle {
                                    Layout.fillWidth: true
                                    Layout.fillHeight: true
                                    radius: 6
                                    color: "#10151a"
                                    border.color: "#2a323b"

                                    ColumnLayout {
                                        anchors.fill: parent
                                        anchors.margins: 12
                                        spacing: 10

                                        Label {
                                            text: "Cover Logo Zones"
                                            color: "#cfd7df"
                                            font.pixelSize: 16
                                        }

                                        Row {
                                            spacing: 10

                                            Repeater {
                                                model: [
                                                    { label: "All", name: "all" },
                                                    { label: "Left", name: "left" },
                                                    { label: "Middle", name: "middle" },
                                                    { label: "Right", name: "right" }
                                                ]

                                                delegate: Button {
                                                    required property var modelData
                                                    text: modelData.label
                                                    implicitWidth: 80
                                                    implicitHeight: 44
                                                    onClicked: {
                                                        root.selectedCoverSegment = modelData.name
                                                        root.selectedCoverSegmentLabel = modelData.label
                                                    }

                                                    background: Rectangle {
                                                        radius: 6
                                                        color: root.selectedCoverSegment === modelData.name ? "#31404e" : "#182028"
                                                        border.color: root.selectedCoverSegment === modelData.name ? "#84a1bc" : "#2a323b"
                                                    }

                                                    contentItem: Label {
                                                        text: parent.text
                                                        color: "#eef2f7"
                                                        font.pixelSize: 14
                                                        font.bold: true
                                                        horizontalAlignment: Text.AlignHCenter
                                                        verticalAlignment: Text.AlignVCenter
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }

                                Rectangle {
                                    Layout.preferredWidth: 240
                                    Layout.fillHeight: true
                                    radius: 6
                                    color: "#10151a"
                                    border.color: "#2a323b"

                                    ColumnLayout {
                                        anchors.fill: parent
                                        anchors.margins: 12
                                        spacing: 10

                                        Label {
                                            text: "Selected: " + root.selectedCoverSegmentLabel
                                            color: "#eef2f7"
                                            font.pixelSize: 16
                                            font.bold: true
                                        }

                                        Label { text: "R"; color: "#cfd7df"; font.pixelSize: 13 }
                                        SpinBox { id: coverRed; from: 0; to: 255; value: 0; editable: true }
                                        Label { text: "G"; color: "#cfd7df"; font.pixelSize: 13 }
                                        SpinBox { id: coverGreen; from: 0; to: 255; value: 0; editable: true }
                                        Label { text: "B"; color: "#cfd7df"; font.pixelSize: 13 }
                                        SpinBox { id: coverBlue; from: 0; to: 255; value: 255; editable: true }
                                        Label { text: "Brightness: " + Math.round(coverBrightness.value); color: "#cfd7df"; font.pixelSize: 13 }
                                        Slider {
                                            id: coverBrightness
                                            from: 0
                                            to: 100
                                            value: 100
                                            stepSize: 1
                                            Layout.fillWidth: true
                                        }

                                        Button {
                                            text: "Apply Brightness"
                                            Layout.fillWidth: true
                                            implicitWidth: 0
                                            Layout.maximumWidth: parent.width - 24
                                            onClicked: lightingUiModel.setCoverLogoBrightness(Math.round(coverBrightness.value))

                                            background: Rectangle {
                                                radius: 6
                                                color: root.testingButtonColor
                                                border.color: root.testingButtonBorder
                                            }

                                            contentItem: Label {
                                                text: parent.text
                                                color: root.testingButtonText
                                                font.pixelSize: 13
                                                font.bold: true
                                                elide: Text.ElideRight
                                                horizontalAlignment: Text.AlignHCenter
                                                verticalAlignment: Text.AlignVCenter
                                            }
                                        }

                                        Button {
                                            text: "Apply Cover Color"
                                            Layout.fillWidth: true
                                            implicitWidth: 0
                                            Layout.maximumWidth: parent.width - 24
                                            onClicked: lightingUiModel.setCoverLogoColor(
                                                root.selectedCoverSegment,
                                                coverRed.value,
                                                coverGreen.value,
                                                coverBlue.value
                                            )

                                            background: Rectangle {
                                                radius: 6
                                                color: root.testingButtonColor
                                                border.color: root.testingButtonBorder
                                            }

                                            contentItem: Label {
                                                text: parent.text
                                                color: root.testingButtonText
                                                font.pixelSize: 13
                                                font.bold: true
                                                elide: Text.ElideRight
                                                horizontalAlignment: Text.AlignHCenter
                                                verticalAlignment: Text.AlignVCenter
                                            }
                                        }
                                    }
                                }
                            }

                            Label {
                                text: "Treat the cover logo as a 3-zone experimental surface until we re-verify left, middle, and right behavior cleanly."
                                color: "#d9b86c"
                                font.pixelSize: 13
                                wrapMode: Text.Wrap
                            }
                        }
                    }

                    Item {
                        ColumnLayout {
                            anchors.fill: parent
                            spacing: 14
                            Label { text: "Base Logo is still in development."; color: "#cfd7df"; font.pixelSize: 16 }
                            Button { text: "Mark Unimplemented"; onClicked: lightingUiModel.noteUnimplemented() }
                        }
                    }

                    Item {
                        ColumnLayout {
                            anchors.fill: parent
                            spacing: 14
                            Label { text: "Infinity Mirror is still in development."; color: "#cfd7df"; font.pixelSize: 16 }
                            Button { text: "Mark Unimplemented"; onClicked: lightingUiModel.noteUnimplemented() }
                        }
                    }
                }

                Rectangle {
                    Layout.fillWidth: true
                    height: 1
                    color: "#2a323b"
                }

                Label {
                    text: "Command History"
                    color: "#f3f4f6"
                    font.pixelSize: 18
                    font.bold: true
                }

                ListView {
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    model: lightingUiModel.history
                    spacing: 10
                    clip: true

                    delegate: Rectangle {
                        required property string title
                        required property string command
                        required property string output
                        required property bool ok

                        width: ListView.view.width
                        radius: 8
                        color: ok ? "#11181c" : "#221619"
                        border.color: ok ? "#2f424d" : "#744247"

                        ColumnLayout {
                            anchors.fill: parent
                            anchors.margins: 12
                            spacing: 6

                            RowLayout {
                                Layout.fillWidth: true
                                Label {
                                    text: title
                                    color: "#eef2f7"
                                    font.pixelSize: 15
                                    font.bold: true
                                    Layout.fillWidth: true
                                }
                                Rectangle {
                                    width: 10
                                    height: 10
                                    radius: 5
                                    color: ok ? "#1f9d55" : "#c24141"
                                }
                            }

                            Text {
                                text: command
                                color: "#9fb0c0"
                                font.family: "monospace"
                                wrapMode: Text.WrapAnywhere
                            }

                            Text {
                                text: output
                                color: "#d1d7de"
                                wrapMode: Text.Wrap
                            }
                        }
                    }
                }

                Label {
                    text: lightingUiModel.status
                    color: "#9fb0c0"
                    Layout.fillWidth: true
                }
            }
        }
    }
}
