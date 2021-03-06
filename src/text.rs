//
// Copyright 2019 Tamas Blummer
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//

//! Encoded text for ads

use std::error::Error;
use std::io::{self, Write, Read, Cursor};
use byteorder::{LittleEndian, WriteBytesExt, ReadBytesExt};

/// A text object that stores a string in space saving encoding
/// currently UTF-8 or UTF-16 with or without snappy compression
#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Clone)]
pub struct Text {
    encoded: Vec<u8>
}

// default encoding is UTF-8 uncompressed
// Below are *bits* of the encoding byte.
// There are thus 4 options:
// 1. Uncompressed UTF-8
// 2. Uncompressed UTF-16
// 3. Compressed UTF-8
// 4. Compressed UTF-16
// uses UTF-16 encoding
const UTF_16:u8 = 1; // bit 0
// uses compressed encoding
const COMPRESSED:u8 = 2; // bit 1

/**
 * The reason why there are so many encoding options is to allow
 * similar messages in different languages to be encoded in a similar
 * number of bytes.
 * Plain UTF-8 is strongly biased towards English and other European
 * languages, while taking more bytes, sometimes many more, for
 * e.g. CJK scripts.
 * Since the serialization length of the advertisements affects how
 * they are ranked, using only uncompressed UTF-8 strongly biases
 * against Asiatic languages.
 * Originally, ZmnSCPxj proposed the use of SCSU encoding, however
 * Tamas counter-proposed this alternative encoding and showed
 * various results with this scheme.
 * SCSU has the disadvantage of not being a widely-accepted
 * standard, and thus there are few implementations of it available
 * out-of-the-box, and thus we would probably have to maintain our
 * own implementation.
 * In any case, an entire byte is allocated for the above two flags,
 * and there is thus room to add even more encodings for text.
 * The intent is that the encodings "on the wire" are not what is
 * exposed to applications that this defiads node is talking to;
 * at higher layers we use UTF-8, but down in the wire level we
 * use various encodings to reduce data size across multiple
 * languages.
 */

impl Text {
    /// create a new text from a string
    pub fn new (s: &str) -> Text {
        let mut flag = 0;
        let mut utf16encoded = Vec::new();
        for utf16 in s.encode_utf16() {
            utf16encoded.write_u16::<LittleEndian>(utf16).unwrap();
        }
        let mut data = if s.len() < utf16encoded.len() {
            s.as_bytes()
        }
        else {
            flag |= UTF_16;
            utf16encoded.as_slice()
        };
        let mut compressor = snap::Writer::new(Vec::new());
        compressor.write_all(data).unwrap();
        let compressed = compressor.into_inner().unwrap();
        if compressed.len() < data.len () {
            flag |= COMPRESSED;
            data = compressed.as_slice();
        }
        let mut encoded = Vec::new();
        encoded.push(flag);
        encoded.extend_from_slice(data);
        Text{encoded}
    }

    pub fn from_encoded(encoded: &[u8]) -> Text {
        Text{encoded: encoded.to_vec()}
    }

    /// return encoded storage
    pub fn as_bytes (&self) -> &[u8] {
        self.encoded.as_slice()
    }

    /// decode the text into a regular string
    pub fn as_string (&self) -> Result<String, Box<dyn Error>> {
        let mut buffer;
        let data = if self.encoded[0] & COMPRESSED != 0 {
            let mut decompressor = snap::Reader::new(io::Cursor::new(&self.encoded[1..]));
            buffer = Vec::new();
            decompressor.read_to_end(&mut buffer)?;
            buffer
        }
        else {
            self.encoded[1..].to_vec()
        };
        if self.encoded[0] & UTF_16 != 0 {
            let mut utf16points = Vec::new();
            let mut cursor = Cursor::new(data);
            while let Ok(utf16) = cursor.read_u16::<LittleEndian>() {
                utf16points.push(utf16);
            }
            Ok(String::from_utf16(utf16points.as_slice())?)
        }
        else {
            Ok(String::from_utf8(data)?)
        }
    }

    /// return the current encoding
    pub fn encoding (&self) -> u8 {
        return self.encoded[0]
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_text () {
        for (language, example) in &EXAMPLES[..] {
            let text = Text::new(example);
            assert_eq!(text.as_string().unwrap(), example.to_string());
            println!("{:20} text encoded in {}{} size compared with UTF-8 {:.1} %", language,
                     if text.encoding() & COMPRESSED != 0 { "compressed " } else {""},
                     if text.encoding() & UTF_16 != 0 {"UTF-16"} else {"UTF-8"},
                     100.0 * (text.as_bytes().len() as f32/example.len() as f32));
        }
    }


    const EXAMPLES: [(&'static str, &'static str); 14] = [
        ("Latin", "Lorem ipsum dolor sit amet, ius te animal perpetua efficiantur, porro dolorem ea mel. Cu duo malorum fastidii delicatissimi, pro dico everti argumentum ex. Ea qui liber solet. Ignota sanctus saperet sea ut, vidisse fuisset eos an. Ius an appareat mediocritatem, eu amet noster reprimique his. Eos in elitr integre mentitum, his fabulas salutatus ea.

Vis duis assum te, eum an nominati expetenda dissentiunt, mei no viris impedit forensibus. Eum lobortis pericula cu, ea persecuti reformidans mea. Eam viris accusam cu, his clita fuisset ex. Alterum oporteat invenire nec no. In erant placerat ponderum mel."),
        ("Cyrillic", "Лорем ипсум долор сит амет, ут сит реяуе елитр реферрентур, деленит инвидунт еррорибус ет меи. Еос легимус цомпрехенсам ад, вих ид аеяуе фацете ехпетенда, фацете оффендит алияуандо ан нец. Но постеа аццусам волуптуа нец, ид мел солута перфецто хонестатис. Ет лаборес молестиае меи. Аццусата еффициенди не вим.

Ан нец ассум ехплицари, долор омиттантур меи ад, усу ут дефинитионес нецесситатибус. Омнис ностер цомпрехенсам вих еи, амет долор сцаевола хис ет. Яуо те медиоцрем ехплицари. Сеа но утамур сцаевола."),
        ("Greek", "Λορεμ ιπσθμ δολορ σιτ αμετ, αν εοσ μοδο σαπερετ σπλενδιδε, ηισ σολθμ vιταε cονσετετθρ εα. Ηισ ει θνθμ cομμθνε ρεπθδιανδαε. Πρι νεμορε μοδερατιθσ σcριβεντθρ εθ, δεβετ ζριλ ηισ νο. Qθοδσι ινιμιcθσ δεμοcριτθμ δθο ατ, μθνδι ρεπθδιανδαε cονσεcτετθερ εστ cθ.

Qθο σολεατ γραεcι μολλισ νο. Εα ιθσ ομνισ λθcιλιθσ περιcθλισ. Εθ cθμ ομνιθμ σινγθλισ cορρθμπιτ, θτ ετιαμ vοcεντ περ. Θτ vοcεντ cονσετετθρ ποσιδονιθμ ηισ. Ηισ θτ ιισqθε προμπτα, ετ σαλε ιλλθμ προβατθσ μει. Qθι αδ εσσε φθγιτ."),
        ("Armenian", "լոռեմ իպսում դոլոռ սիթ ամեթ, իդ եոս մինիմ մելիուս սենթենթիաե, նե լաուդեմ ծոնսթիթութո եամ. եոս ութ ամեթ իգնոթա քուոդսի. աթ աուդիռե լեգիմուս վիխ, նիսլ պեռթինածիա նո պռո. նամ իդ ոպթիոն պառթիենդո պեռսեքուեռիս, եում լաբոռե սալութաթուս սծռիբենթուռ եա. իդ վիմ եռռոռ լաոռեեթ դիսպութաթիոնի, թե մունդի պլածեռաթ սենթենթիաե նամ, եթ սիմուլ դեծոռե ոպոռթեաթ քուո.

մալոռում վոծիբուս դոլոռես ադ վել, նե մեա մոլլիս վիդիսսե, իուս նո թալե հինծ ոֆֆենդիթ. նուլլա վիդեռեռ պաթռիոքուե վիս եու, եում ծու լաբոռե ֆասթիդիի. եում թե պեռծիպիթ ուռբանիթաս ծոնծլուսիոնեմքուե, եա մել մովեթ լեգիմուս ծոռպոռա."),
        ("Georgian", "ლორემ იფსუმ დოლორ სით ამეთ, ფრიმის სენსიბუს ეუ ცუმ. დუო ფუთანთ სალუთათუს ცონსეყუუნთურ ეხ, ათყუი სემფერ ლეგენდოს ყუო ად, აფერიამ ათომორუმ ომითთანთურ ეუმ ეი. ეამ ად ილლუდ დიცთას, ეი დიცამ სადიფსცინგ ესთ. გრაეცო თიბიყუე ეამ ად, მეა უთ ილლუმ ნოსთერ მანდამუს. ინ მეა აუდიამ ფრობათუს, სეა უთინამ ერიფუით დესერუნთ ცუ.

ვერთერემ ლეგენდოს მოდერათიუს მეა ინ, ნო ვის რებუმ ევერთი ფროდესსეთ. დელენით დეთრაცთო ჰის ეთ, ცუ მეის დიცთა ფრი. ეუ უსუ ცონცეფთამ დესერუისსე. სით ეთ ეუისმოდ სალუთანდი. თე ეუმ უნუმ ფოფულო ვოცენთ, უსუ დიცუნთ თინციდუნთ ნო, ნამ ობლიყუე ვოლუფთარია ეთ. ეამ ან ესსენთ ლაბორეს ცომმუნე, ან ინვენირე ფერთინაცია სეა."),
        ("Hindi", "गुजरना हार्डवेर कार्यसिधान्तो भाषए संसाध सके। दौरान तकरीबन विश्वास औषधिक बिन्दुओ कार्यलय सेऔर लिए। प्रतिबध विश्लेषण समाजो लाभो अधिकार अधिकांश शीघ्र हमारी परिवहन बीसबतेबोध अर्थपुर्ण जोवे बाटते जैसे हुआआदी उसीएक् विश्लेषण जिवन उद्योग अधिकांश विशेष कीसे सुविधा दस्तावेज बिन्दुओमे मुख्य भारतीय सुस्पश्ट अमितकुमार खरिदे सुनत

कार्यलय सोफ़्टवेर सुना लेकिन सेऔर तरीके उद्योग सभिसमज मुख्यतह विश्व परिभाषित वार्तालाप कीसे जोवे वर्णन आंतरजाल असरकारक विकेन्द्रित नाकर हमारि क्षमता है।अभी ब्रौशर है।अभी बाजार करती रखति संदेश वर्णित विश्वास मुश्किले तकनीकी अंतर्गत होगा हिंदी विभाग मेमत दिनांक पहोचाना कोहम विभाग पुर्णता उपयोगकर्ता"),
        ("Chinese", "職認子相帯金領観年旅計読。東率歳本読谷車陸保美情僕代捕期負骨義著一。嫌経企業補身定倍国夫同裁損併。法初勝載的図午馬能権紙相報謝年子禁可。全断素済禁間宮松協関抱田過問接青。徴側患上病先泊試競践味保通著組質市伸知前。付祖越一細済現境属堀約天扱知活必面高反。位式座堪米軍手子分川旅出。無独港前度万奈山国団能署画翔基全覚雪質。

旅面展退賃買系身者欲力経断判視対。変不禁明治確統言男害愛寺松目日。会死順務船覧時年朝事辺電線。連木作引察力代番名著築秘元歌跡多。不並何頭台必来理分地研行芸高。口所葉申親初治回空品法供校当。力病年却与位伝供待心月月情能児想。効設軽裁球段夢左強長約明端竹郁。橋英自富際院極図瀬部度車良命謙次政問億意。"),
        ("Japanese-1", "名げではん意問ぴレぶよ輝点ゆぎ全平方ミカハ国健労みまドよ役著世すい細団や田南コト版助たのフ留貢トウホ食総ばぴ担少9形ミネヘセ香加ルらん写裏玲ご山潤むへけこ。殺あみ通変メソワヲ毎格リ現制ロサミ保個フシサロ明毎許セチレテ石3断式ずろ一7応ホカ定設タムニ時孝クの。分をレル島役やトおひ整期ハシキヤ伊特千ばみ写載郎げぽご路攻ユ入出げ合割カ新36座ウハチ追省とみなむ舎29捜いれ。

忘ーろた質問枚題マセ申席催スふ答無トフエ絹予福エチメユ見4株太よ写点ラち閣七ゅわ告取躍か。周ゆら権南ヒトレ成週テソ剱59富そゅめ配可スシリナ兵査ルノナ習済室ぐ流暮ぼ必聞とま本決キ続年ヲレモ皇更ルニウ行除ゆばこぽ広残ツチス転政くにるン感米えふぴし賞撤い。目ぐあド沢報めうざ必新ホラヨ色24昇処輪傑20件ヲシ断投ばやに殺9択海求ふ題発年モヨ町無スクソム手人がと必群蔵止求らフけ。"),
        ("Japanese-2", "う遊他派とょせ他けれっなっへるくら。しこちねたゃ、都樹んっむ尾いゅさ譜野舳知名課ゃそひにはよた津以舳都ろりっもセスノロヤセユヨホまちて保名こるスイヒ遊都、んさめぬぬに。きく、個二樹雲離離ナメメスョゅゅね、るへれ遊個いま。さえ、の保列よええちそ尾鵜都露保樹なゅつとほょらこみい魔絵ょ無舳ゆせ阿あソュセハッ。

瀬差津しにみえひら、かちるる二、野露目露おなとっひ、ヨチョテ課素ホュニテ雲津保みらよんよまし個っ夜差根目等もへえけほょ、派夜根知保、差こ阿素個遊樹他えなぬもよかゅ無つれ、き。"),
        ("Japanese-Jugemu", "シューリンガンのグーリンダイ。五劫の擦り切れ。やぶら小路の藪柑子。海砂利水魚の、グーリンダイのポンポコピーのポンポコナーの、寿限無、寿限無。水行末 雲来末 風来末、長久命の長助。パイポパイポ パイポのシューリンガン。シューリンガンのグーリンダイ。

シューリンガンのグーリンダイ、食う寝る処に住む処。長久命の長助。寿限無、寿限無。グーリンダイのポンポコピーのポンポコナーの、五劫の擦り切れ。水行末 雲来末 風来末、五劫の擦り切れ、グーリンダイのポンポコピーのポンポコナーの、やぶら小路の藪柑子。水行末 雲来末 風来末、寿限無、寿限無、シューリンガンのグーリンダイ。やぶら小路の藪柑子、長久命の長助、パイポパイポ パイポのシューリンガン。海砂利水魚の。海砂利水魚の。"),
        ("Korean-1", "모든 국민은 신속한 재판을 받을 권리를 가진다. 대통령은 취임에 즈음하여 다음의 선서를 한다. 내부규율과 사무처리에 관한 규칙을 제정할 수 있다. 다만.

1차에 한하여 중임할 수 있다. 모든 국민은 행위시의 법률에 의하여 범죄를 구성하지 아니하는 행위로 소추되지 아니하며, 사영기업을 국유 또는 공유로 이전하거나 그 경영을 통제 또는 관리할 수 없다. 국회는 국민의 보통·평등·직접·비밀선거에 의하여 선출된 국회의원으로 구성한다."),
        ("Korean-2", "풀밭에 이 같은 이것이다. 피가 사랑의 보이는 얼마나 가진 찾아 아니다. 싶이 운다. 있다. 넣는 거친 수 맺어, 있다. 불어 그들은 위하여서, 눈이 일월과 열락의 목숨이 남는 내려온 힘있다.

지혜는 황금시대를 그들은 인생을 천자만홍이 자신과 사막이다, 청춘에서만 오아이스도 천자만홍이 기관과 황금시대다. 수 이것이다, 얼마나 끓는다, 가슴에 어디 피고. 날카로우나 없는 그러므로 봄바람이다. 방지하는 이상은 길지 발휘하기 희망의 가는 일월과 쓸쓸하랴?광야에서 있는 이는 수 힘차게 수 품고 이상. 능히 그들은 것은 보배를 이상의 것이다."),
        ("Arabic", "إذ الحرة الإطلاق يبق, إذ حول قتيل، ارتكبها الرئيسية. دار مشروط فاتّبع بـ, تعد أوسع قبضتهم تكاليف أن. قدما وإقامة في عدم, غير تم ويعزى التخطيط. لكل وقوعها، المتحدة الدولارات ان, ليبين أعلنت التبرعات دنو بـ.

إذ يكن بتحدّي وفنلندا. ومن ٣٠ البرية الجديدة،. وقام انذار أي جُل, عدم فكانت ضمنها وايرلندا كل. عن به، نهاية وبولندا, الى لهيمنة بريطانيا-فرنسا إذ."),
        ("Hebrew", "חפש יכול למאמרים אל, כלל ראשי חינוך את. רפואה פילוסופיה גם זכר, אינטרנט מועמדים דת מלא. שכל אל זקוק טיפול, אל המחשב המזנון הספרות בקר, מתן אל ספרדית מדריכים ותשובות. מוגש מדויקים שכל מה.

ארץ מה וקשקש מונחים תחבורה, אל פנאי לערוך היא, ויקי פיסול גם קרן. ב לערכים ומהימנה לימודים היא. ספרות שיתופית ליצירתה מה אחר, גם העיר חינוך עזה, לציין משופרות אם צ'ט. דפים טכנולוגיה צ'ט או, אם עוד שימושי וספציפיים. לוח מה יכול שנורו לעריכה."),
    ];


}
